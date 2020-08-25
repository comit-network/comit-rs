use crate::{
    config::Settings,
    ethereum, hbit, herc20,
    local_swap_id::LocalSwapId,
    network::peer_tracker::PeerTracker,
    spawn,
    storage::{CreatedSwap, ForSwap, Load, RootSeed, Save, Storage, SwapContext},
    ProtocolSpawner,
};
use chrono::Utc;
use comit::{
    asset, lightning,
    network::{
        comit::{Comit, LocalData, RemoteData},
        orderbook,
        orderbook::Orderbook,
        protocols::{
            announce,
            announce::Announce,
            setup_swap::{CommonParams, SetupSwap},
        },
        setup_swap,
        swap_digest::SwapDigest,
        Identities, SharedSwapId, WhatAliceLearnedFromBob, WhatBobLearnedFromAlice,
    },
    order::SwapProtocol,
    orderpool, BtcDaiOrderForm, LockProtocol, Never, OrderId, Position, Role, SecretHash, Side,
};
use futures::TryFutureExt;
use libp2p::{identity::Keypair, NetworkBehaviour, PeerId};
use std::collections::HashMap;
use tokio::runtime::Handle;

/// A `NetworkBehaviour` that represents a COMIT node.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode {
    pub announce: Announce<LocalSwapId>,
    pub setup_swap: SetupSwap<LocalSwapId>,
    pub orderbook: Orderbook,
    pub comit: Comit,
    pub peer_tracker: PeerTracker,

    #[behaviour(ignore)]
    seed: RootSeed,
    #[behaviour(ignore)]
    task_executor: Handle,
    /// We receive the LocalData for the execution parameter exchange at the
    /// same time as we announce the swap. We save `LocalData` here until the
    /// swap is confirmed.
    #[behaviour(ignore)]
    local_data: HashMap<LocalSwapId, LocalData>,
    /// The execution parameter exchange only knows about `SharedSwapId`s, so we
    /// need to map this back to a `LocalSwapId` to save the data correctly to
    /// the database.
    #[behaviour(ignore)]
    local_swap_ids: HashMap<SharedSwapId, LocalSwapId>,
    #[behaviour(ignore)]
    order_addresses: HashMap<OrderId, BtcDaiOrderAddresses>,
    #[behaviour(ignore)]
    storage: Storage,
    #[behaviour(ignore)]
    protocol_spawner: ProtocolSpawner,
    /// The Bitcoin network we are currently connected to.
    #[behaviour(ignore)]
    _configured_bitcoin_network: bitcoin::Network,
    /// The Ethereum network we are currently connected to.
    #[behaviour(ignore)]
    _configured_ethereum_network: ethereum::ChainId,
    /// The address of the DAI ERC20 token contract on the current Ethereum
    /// network.
    #[behaviour(ignore)]
    _dai_contract_address: ethereum::Address,
}

/// A container used for temporarily storing two addresses provided by a user.
///
/// These addresses are used to set up a swap once the order they are associated
/// with yields a match. This allows us to avoid an interaction with the user at
/// the time of an order match.
#[derive(Debug)]
pub struct BtcDaiOrderAddresses {
    pub bitcoin: bitcoin::Address,
    pub ethereum: ethereum::Address,
}

impl ComitNode {
    pub fn new(
        seed: RootSeed,
        task_executor: Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
        peer_id: PeerId,
        key: Keypair,
        settings: &Settings,
    ) -> Self {
        Self {
            announce: Announce::default(),
            setup_swap: Default::default(),
            orderbook: Orderbook::new(peer_id, key),
            comit: Comit::default(),
            peer_tracker: PeerTracker::default(),
            seed,
            task_executor,
            local_data: HashMap::default(),
            local_swap_ids: HashMap::default(),
            order_addresses: Default::default(),
            storage,
            protocol_spawner,
            _configured_bitcoin_network: settings.bitcoin.network,
            _configured_ethereum_network: settings.ethereum.chain_id,
            _dai_contract_address: settings.ethereum.tokens.dai,
        }
    }

    pub fn initiate_communication(
        &mut self,
        local_swap_id: LocalSwapId,
        peer_id: PeerId,
        role: Role,
        digest: SwapDigest,
        identities: Identities,
    ) -> anyhow::Result<()> {
        // At this stage we do not know if the arguments passed to us make up a
        // valid swap, we just trust the controller to pass in something
        // valid. Do _some_ form of validation here so that we can early return
        // errors and they do not get lost in the asynchronous call chain that
        // kicks off here.
        self.assert_have_lnd_if_needed(identities.lightning_identity)?;

        let local_data = match role {
            Role::Alice => {
                self.announce.announce_swap(digest, peer_id, local_swap_id);

                let swap_seed = self.seed.derive_swap_seed(local_swap_id);
                let secret = swap_seed.derive_secret();
                let secret_hash = SecretHash::new(secret);

                LocalData::for_alice(secret_hash, identities)
            }
            Role::Bob => {
                self.announce
                    .await_announcement(digest, peer_id, local_swap_id);

                LocalData::for_bob(identities)
            }
        };

        self.local_data.insert(local_swap_id, local_data);

        Ok(())
    }

    pub fn publish_order(
        &mut self,
        form: BtcDaiOrderForm,
        addresses: BtcDaiOrderAddresses,
    ) -> OrderId {
        let id = self.orderbook.publish(form);
        self.order_addresses.insert(id, addresses);

        id
    }

    fn assert_have_lnd_if_needed(
        &self,
        identity: Option<lightning::PublicKey>,
    ) -> anyhow::Result<()> {
        if identity.is_some() {
            return self.protocol_spawner.supports_halbit();
        }
        Ok(())
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ComitNode {
    fn inject_event(&mut self, _event: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<Never> for ComitNode {
    fn inject_event(&mut self, _: Never) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<::comit::network::comit::BehaviourOutEvent>
    for ComitNode
{
    fn inject_event(&mut self, event: ::comit::network::comit::BehaviourOutEvent) {
        match event {
            ::comit::network::comit::BehaviourOutEvent::SwapFinalized {
                shared_swap_id,
                remote_data,
            } => {
                let storage = self.storage.clone();
                let spawner = self.protocol_spawner.clone();

                let local_swap_id = match self.local_swap_ids.remove(&shared_swap_id) {
                    Some(local_swap_id) => local_swap_id,
                    None => {
                        tracing::warn!("inconsistent data, missing local_swap_id mapping");
                        return;
                    }
                };

                let save_and_start_swap = async move {
                    let swap = storage.load(local_swap_id).await?;
                    save_swap_remote_data(&storage, swap, remote_data).await?;
                    spawn::spawn(&spawner, &storage, swap).await?;

                    Ok::<(), anyhow::Error>(())
                };

                self.task_executor
                    .spawn(save_and_start_swap.map_err(|e: anyhow::Error| {
                        tracing::error!("{}", e);
                    }));
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<announce::BehaviourOutEvent<LocalSwapId>>
    for ComitNode
{
    fn inject_event(&mut self, event: announce::BehaviourOutEvent<LocalSwapId>) {
        match event {
            announce::BehaviourOutEvent::Confirmed {
                peer,
                shared_swap_id,
                context: local_swap_id,
            } => {
                let data = match self.local_data.remove(&local_swap_id) {
                    Some(local_data) => local_data,
                    None => {
                        tracing::warn!("inconsistent data, missing local-data mapping");
                        return;
                    }
                };

                self.comit.communicate(peer, shared_swap_id, data);
                self.local_swap_ids.insert(shared_swap_id, local_swap_id);
            }
            announce::BehaviourOutEvent::Failed {
                peer,
                context: local_swap_id,
            } => {
                tracing::warn!(
                    "failed to complete announce protocol for swap {} with {}",
                    local_swap_id,
                    peer,
                );
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<orderbook::BehaviourOutEvent> for ComitNode {
    fn inject_event(&mut self, event: orderbook::BehaviourOutEvent) {
        match event {
            orderbook::BehaviourOutEvent::OrderMatch(orderpool::Match {
                price,
                quantity,
                ours,
                theirs,
                peer,
                our_position,
                swap_protocol,
                match_reference_point,
            }) => {
                tracing::info!(
                    "order {} matched against order {} from {}",
                    ours,
                    theirs,
                    peer
                );
                // todo: remove this unwrap
                let our_identities = self.order_addresses.get(&ours).unwrap();
                let token_contract = self._dai_contract_address;
                let swap_id = LocalSwapId::random();
                let swap_seed = self.seed.derive_swap_seed(swap_id);

                match swap_protocol {
                    SwapProtocol::HbitHerc20 {
                        hbit_expiry_offset,
                        herc20_expiry_offset,
                    } => {
                        // todo: do checked addition
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let (ethereum_absolute_expiry, bitcoin_absolute_expiry) = {
                            let ethereum =
                                (match_reference_point + herc20_expiry_offset).timestamp() as u32;
                            let bitcoin =
                                (match_reference_point + hbit_expiry_offset).timestamp() as u32;

                            (ethereum, bitcoin)
                        };
                        let erc20_quantity = quantity * price;
                        match our_position {
                            Position::Buy => {
                                if let Err(e) = self.setup_swap.bob_send_hbit_herc20(
                                    &peer,
                                    self.storage.derive_transient_identity(
                                        swap_id,
                                        Role::Bob,
                                        Side::Alpha,
                                    ),
                                    our_identities.ethereum,
                                    CommonParams {
                                        erc20: asset::Erc20 {
                                            token_contract,
                                            quantity: erc20_quantity,
                                        },
                                        bitcoin: quantity,
                                        ethereum_absolute_expiry,
                                        bitcoin_absolute_expiry,
                                        ethereum_chain_id: self._configured_ethereum_network,
                                        bitcoin_network: self._configured_bitcoin_network,
                                    },
                                    ours,
                                    swap_id,
                                ) {
                                    tracing::warn!("{}", e);
                                }
                            }
                            Position::Sell => {
                                let secret = swap_seed.derive_secret();
                                let secret_hash = SecretHash::new(secret);

                                if let Err(e) = self.setup_swap.alice_send_hbit_herc20(
                                    &peer,
                                    self.storage.derive_transient_identity(
                                        swap_id,
                                        Role::Alice,
                                        Side::Alpha,
                                    ),
                                    our_identities.ethereum,
                                    secret_hash,
                                    CommonParams {
                                        erc20: asset::Erc20 {
                                            token_contract: self._dai_contract_address,
                                            quantity: erc20_quantity,
                                        },
                                        bitcoin: quantity,
                                        ethereum_absolute_expiry,
                                        bitcoin_absolute_expiry,
                                        ethereum_chain_id: self._configured_ethereum_network,
                                        bitcoin_network: self._configured_bitcoin_network,
                                    },
                                    ours,
                                    swap_id,
                                ) {
                                    tracing::warn!("{}", e);
                                }
                            }
                        }
                    }
                    SwapProtocol::Herc20Hbit {
                        hbit_expiry_offset,
                        herc20_expiry_offset,
                    } => {
                        // todo: do checked addition
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let (ethereum_absolute_expiry, bitcoin_absolute_expiry) = {
                            let ethereum =
                                (match_reference_point + herc20_expiry_offset).timestamp() as u32;
                            let bitcoin =
                                (match_reference_point + hbit_expiry_offset).timestamp() as u32;

                            (ethereum, bitcoin)
                        };
                        let erc20_quantity = quantity * price;
                        match our_position {
                            Position::Buy => {
                                let secret = swap_seed.derive_secret();
                                let secret_hash = SecretHash::new(secret);

                                if let Err(e) = self.setup_swap.alice_send_herc20_hbit(
                                    &peer,
                                    self.storage.derive_transient_identity(
                                        swap_id,
                                        Role::Alice,
                                        Side::Beta,
                                    ),
                                    our_identities.ethereum,
                                    secret_hash,
                                    CommonParams {
                                        erc20: asset::Erc20 {
                                            token_contract,
                                            quantity: erc20_quantity,
                                        },
                                        bitcoin: quantity,
                                        ethereum_absolute_expiry,
                                        bitcoin_absolute_expiry,
                                        ethereum_chain_id: self._configured_ethereum_network,
                                        bitcoin_network: self._configured_bitcoin_network,
                                    },
                                    ours,
                                    swap_id,
                                ) {
                                    tracing::warn!("{}", e);
                                }
                            }
                            Position::Sell => {
                                if let Err(e) = self.setup_swap.bob_send_herc20_hbit(
                                    &peer,
                                    self.storage.derive_transient_identity(
                                        swap_id,
                                        Role::Bob,
                                        Side::Beta,
                                    ),
                                    our_identities.ethereum,
                                    CommonParams {
                                        erc20: asset::Erc20 {
                                            token_contract: self._dai_contract_address,
                                            quantity: erc20_quantity,
                                        },
                                        bitcoin: quantity,
                                        ethereum_absolute_expiry,
                                        bitcoin_absolute_expiry,
                                        ethereum_chain_id: self._configured_ethereum_network,
                                        bitcoin_network: self._configured_bitcoin_network,
                                    },
                                    ours,
                                    swap_id,
                                ) {
                                    tracing::warn!("{}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<setup_swap::BehaviourOutEvent<LocalSwapId>>
    for ComitNode
{
    fn inject_event(&mut self, event: setup_swap::BehaviourOutEvent<LocalSwapId>) {
        match event {
            setup_swap::BehaviourOutEvent::ExecutableSwap(exec_swap) => {
                let spawner = self.protocol_spawner.clone();

                let swap_id = exec_swap.context;
                let secret_hash = exec_swap.herc20.secret_hash;

                // todo: remove unwrap
                let bitcoin_address = self
                    .order_addresses
                    .get(&exec_swap.order_id)
                    .unwrap()
                    .bitcoin
                    .clone();

                let remote_data = RemoteData {
                    secret_hash: Some(secret_hash),
                    ethereum_identity: Some(exec_swap.their_identities().ethereum),
                    lightning_identity: None,
                    bitcoin_identity: Some(exec_swap.their_identities().bitcoin),
                };

                match exec_swap.swap_protocol {
                    setup_swap::SwapProtocol::HbitHerc20 => {
                        let swap_ctx = SwapContext {
                            id: swap_id,
                            role: exec_swap.my_role,
                            alpha: LockProtocol::Hbit,
                            beta: LockProtocol::Herc20,
                        };

                        let created_swap = CreatedSwap {
                            swap_id,
                            alpha: hbit::CreatedSwap {
                                amount: exec_swap.hbit.asset,
                                final_identity: bitcoin_address,
                                network: exec_swap.hbit.network.into(),
                                absolute_expiry: exec_swap.hbit.expiry.into(),
                            },
                            beta: herc20::CreatedSwap {
                                asset: exec_swap.herc20.asset.clone(),
                                identity: exec_swap.my_identities().ethereum,
                                chain_id: exec_swap.herc20.chain_id,
                                absolute_expiry: exec_swap.herc20.expiry.into(),
                            },
                            peer: exec_swap.peer_id,
                            address_hint: None,
                            role: exec_swap.my_role,
                            // todo: change to match ref point
                            start_of_swap: Utc::now().naive_utc(),
                        };

                        let storage = self.storage.clone();

                        let start_swap = async move {
                            storage.save(created_swap).await?;
                            hack_secret_hash_into_db(swap_id, secret_hash, &storage).await?;
                            save_swap_remote_data(&storage, swap_ctx, remote_data).await?;
                            spawn::spawn(&spawner, &storage, swap_ctx).await?;
                            Ok::<(), anyhow::Error>(())
                        };

                        self.task_executor
                            .spawn(start_swap.map_err(|e: anyhow::Error| {
                                tracing::error!("{}", e);
                            }));
                    }
                    setup_swap::SwapProtocol::Herc20Hbit => {
                        let swap_ctx = SwapContext {
                            id: swap_id,
                            role: exec_swap.my_role,
                            alpha: LockProtocol::Herc20,
                            beta: LockProtocol::Hbit,
                        };

                        let created_swap = CreatedSwap {
                            swap_id,
                            beta: hbit::CreatedSwap {
                                amount: exec_swap.hbit.asset,
                                final_identity: bitcoin_address,
                                network: exec_swap.hbit.network.into(),
                                absolute_expiry: exec_swap.hbit.expiry.into(),
                            },
                            alpha: herc20::CreatedSwap {
                                asset: exec_swap.herc20.asset.clone(),
                                identity: exec_swap.my_identities().ethereum,
                                chain_id: exec_swap.herc20.chain_id,
                                absolute_expiry: exec_swap.herc20.expiry.into(),
                            },
                            peer: exec_swap.peer_id.clone(),
                            address_hint: None,
                            role: exec_swap.my_role,
                            // todo: change to match ref point
                            start_of_swap: Utc::now().naive_utc(),
                        };

                        let storage = self.storage.clone();

                        let start_swap = async move {
                            storage.save(created_swap).await?;
                            hack_secret_hash_into_db(swap_id, secret_hash, &storage).await?;
                            save_swap_remote_data(&storage, swap_ctx, remote_data).await?;
                            spawn::spawn(&spawner, &storage, swap_ctx).await?;
                            Ok::<(), anyhow::Error>(())
                        };

                        self.task_executor
                            .spawn(start_swap.map_err(|e: anyhow::Error| {
                                tracing::error!("{}", e);
                            }));
                    }
                };
            }
            setup_swap::BehaviourOutEvent::AlreadyHaveRoleParams { peer, .. } => tracing::error!(
                "Already have role dependent parameters from this peer: {}",
                peer
            ),
        }
    }
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error(
"unable to save swap with id {local_swap_id} in database because the protocol combination is not supported"
)]
struct SaveUnsupportedSwap {
    local_swap_id: LocalSwapId,
}

async fn save_swap_remote_data(
    storage: &Storage,
    swap: SwapContext,
    data: RemoteData,
) -> anyhow::Result<()> {
    match (&swap, data) {
        (
            SwapContext {
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Halbit,
                role: Role::Alice,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: ethereum_identity,
                        beta_refund_identity: lightning_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Halbit,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: ethereum_identity,
                        beta_redeem_identity: lightning_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: LockProtocol::Halbit,
                beta: LockProtocol::Herc20,
                role: Role::Alice,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: lightning_identity,
                        beta_refund_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: LockProtocol::Halbit,
                beta: LockProtocol::Herc20,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: lightning_identity,
                        beta_redeem_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Hbit,
                role: Role::Alice,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: ethereum_identity,
                        beta_refund_identity: bitcoin_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Hbit,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: ethereum_identity,
                        beta_redeem_identity: bitcoin_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: LockProtocol::Hbit,
                beta: LockProtocol::Herc20,
                role: Role::Alice,
                ..
            },
            RemoteData {
                bitcoin_identity: Some(bitcoin_identity),
                ethereum_identity: Some(ethereum_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: bitcoin_identity,
                        beta_refund_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: LockProtocol::Hbit,
                beta: LockProtocol::Herc20,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: bitcoin_identity,
                        beta_redeem_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        _ => anyhow::bail!(SaveUnsupportedSwap {
            local_swap_id: swap.id,
        }),
    };

    Ok(())
}

// extremely dirty hack to work around the problem that Alice doesn't update the
// secret hash in `save_swap_remote_data` previously, we had saved that already
// prior to this call but with setup-swap, both parties learn about the secret
// hash at the same time
async fn hack_secret_hash_into_db(
    swap_id: LocalSwapId,
    secret_hash: SecretHash,
    storage: &Storage,
) -> anyhow::Result<()> {
    storage
        .db
        .do_in_transaction(|conn| storage.db.insert_secret_hash(conn, swap_id, secret_hash))
        .await?;

    Ok(())
}
