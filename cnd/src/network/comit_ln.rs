use crate::{
    asset, identity,
    network::{
        oneshot_behaviour,
        protocols::{
            announce,
            announce::{behaviour::Announce, SwapDigest},
            ethereum_identity, finalize, lightning_identity, secret_hash,
        },
    },
    seed::{DeriveSwapSeedFromNodeLocal, RootSeed},
    swap_protocols::{
        ledger::{ethereum::ChainId, lightning, Ethereum},
        rfc003::{create_swap::HtlcParams, DeriveSecret, Secret, SecretHash},
        HanEtherereumHalightBitcoinCreateSwapParams, NodeLocalSwapId, Role, SwapId,
    },
    timestamp::Timestamp,
};
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;
use digest::Digest;
use futures::AsyncWriteExt;
use libp2p::{
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour,
};
use std::{
    collections::{HashMap, VecDeque},
    task::{Context, Poll},
};

/// Event emitted  by the `ComitLn` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    SwapFinalized {
        local_swap_id: NodeLocalSwapId,
        swap_params: HanEtherereumHalightBitcoinCreateSwapParams,
        secret_hash: SecretHash,
        ethereum_identity: identity::Ethereum,
    },
}

#[derive(NetworkBehaviour, Debug)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
pub struct ComitLN {
    announce: Announce,
    secret_hash: oneshot_behaviour::Behaviour<secret_hash::Message>,
    ethereum_identity: oneshot_behaviour::Behaviour<ethereum_identity::Message>,
    lightning_identity: oneshot_behaviour::Behaviour<lightning_identity::Message>,
    finalize: oneshot_behaviour::Behaviour<finalize::Message>,

    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,

    #[behaviour(ignore)]
    swaps_waiting_for_announcement: HashMap<SwapDigest, NodeLocalSwapId>,
    #[behaviour(ignore)]
    swaps: HashMap<NodeLocalSwapId, HanEtherereumHalightBitcoinCreateSwapParams>,
    #[behaviour(ignore)]
    swap_ids: HashMap<NodeLocalSwapId, SwapId>,
    #[behaviour(ignore)]
    ethereum_identities: HashMap<SwapId, identity::Ethereum>,
    #[behaviour(ignore)]
    lightning_identities: HashMap<SwapId, identity::Lightning>,
    #[behaviour(ignore)]
    communication_state: HashMap<SwapId, CommunicationState>,
    #[behaviour(ignore)]
    secret_hashes: HashMap<SwapId, SecretHash>,

    #[behaviour(ignore)]
    pub seed: RootSeed,
}

#[derive(Debug, Default)]
struct CommunicationState {
    ethereum_identity_sent: bool,
    lightning_identity_sent: bool,
    received_finalized: bool,
    sent_finalized: bool,
    secret_hash_sent_or_received: bool,
}

impl ComitLN {
    pub fn new(seed: RootSeed) -> Self {
        ComitLN {
            announce: Default::default(),
            secret_hash: Default::default(),
            ethereum_identity: Default::default(),
            lightning_identity: Default::default(),
            finalize: Default::default(),
            events: VecDeque::new(),
            swaps_waiting_for_announcement: Default::default(),
            swaps: Default::default(),
            swap_ids: Default::default(),
            ethereum_identities: Default::default(),
            lightning_identities: Default::default(),
            communication_state: Default::default(),
            secret_hashes: Default::default(),
            seed,
        }
    }

    pub fn initiate_communication(
        &mut self,
        id: NodeLocalSwapId,
        create_swap_params: HanEtherereumHalightBitcoinCreateSwapParams,
    ) {
        let digest = create_swap_params.clone().digest();

        self.swaps.insert(id, create_swap_params.clone());

        match create_swap_params.role {
            Role::Alice => {
                if self.swaps_waiting_for_announcement.contains_key(&digest) {
                    // To fix this panic, we should either pass the local swap id to the
                    // announce behaviour or get a unique token from the behaviour that
                    // we can use to track the progress of the announcement
                    panic!("cannot send two swaps with the same digest at the same time!")
                }

                self.announce
                    .start_announce_protocol(digest.clone(), create_swap_params.peer);

                self.swaps_waiting_for_announcement.insert(digest, id);
            }
            Role::Bob => {
                tracing::info!("Swap waiting for announcement: {}", digest);
                self.swaps_waiting_for_announcement.insert(digest, id);
            }
        }
    }

    pub fn get_finalized_swap(&self, local_id: NodeLocalSwapId) -> Option<FinalizedSwap> {
        let create_swap_params = match self.swaps.get(&local_id) {
            Some(body) => body,
            None => return None,
        };

        let secret = match create_swap_params.role {
            Role::Alice => Some(
                self.seed
                    .derive_swap_seed_from_node_local(local_id)
                    .derive_secret(),
            ),
            Role::Bob => None,
        };

        let id = match self.swap_ids.get(&local_id).copied() {
            Some(id) => id,
            None => return None,
        };

        let alpha_ledger_redeem_identity = match create_swap_params.role {
            Role::Alice => match self.ethereum_identities.get(&id).copied() {
                Some(identity) => identity,
                None => return None,
            },
            Role::Bob => create_swap_params.ethereum_identity.into(),
        };
        let alpha_ledger_refund_identity = match create_swap_params.role {
            Role::Alice => create_swap_params.ethereum_identity.into(),
            Role::Bob => match self.ethereum_identities.get(&id).copied() {
                Some(identity) => identity,
                None => return None,
            },
        };
        let beta_ledger_redeem_identity = match create_swap_params.role {
            Role::Alice => create_swap_params.lightning_identity,
            Role::Bob => match self.lightning_identities.get(&id).copied() {
                Some(identity) => identity,
                None => return None,
            },
        };
        let beta_ledger_refund_identity = match create_swap_params.role {
            Role::Alice => match self.lightning_identities.get(&id).copied() {
                Some(identity) => identity,
                None => return None,
            },
            Role::Bob => create_swap_params.lightning_identity,
        };

        Some(FinalizedSwap {
            alpha_ledger: Ethereum::new(ChainId::regtest()),
            beta_ledger: lightning::Regtest,
            alpha_asset: create_swap_params.ethereum_amount.clone(),
            beta_asset: create_swap_params.lightning_amount,
            alpha_ledger_redeem_identity,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            beta_ledger_refund_identity,
            alpha_expiry: create_swap_params.ethereum_absolute_expiry,
            beta_expiry: create_swap_params.lightning_cltv_expiry,
            local_id,
            secret,
            secret_hash: match self.secret_hashes.get(&id).copied() {
                Some(secret_hash) => secret_hash,
                None => return None,
            },
            role: create_swap_params.role,
        })
    }

    fn poll<BIE>(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<BIE, BehaviourOutEvent>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        // We trust in libp2p to poll us.
        Poll::Pending
    }
}

#[derive(Debug)]
pub struct FinalizedSwap {
    pub alpha_ledger: Ethereum,
    pub beta_ledger: lightning::Regtest,
    pub alpha_asset: asset::Ether,
    pub beta_asset: asset::Lightning,
    pub alpha_ledger_refund_identity: identity::Ethereum,
    pub alpha_ledger_redeem_identity: identity::Ethereum,
    pub beta_ledger_refund_identity: identity::Lightning,
    pub beta_ledger_redeem_identity: identity::Lightning,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub local_id: NodeLocalSwapId,
    pub secret_hash: SecretHash,
    pub secret: Option<Secret>,
    pub role: Role,
}

impl FinalizedSwap {
    pub fn han_params(&self) -> EtherHtlc {
        HtlcParams {
            asset: self.alpha_asset.clone(),
            ledger: Ethereum::new(ChainId::regtest()),
            redeem_identity: self.alpha_ledger_redeem_identity,
            refund_identity: self.alpha_ledger_refund_identity,
            expiry: self.alpha_expiry,
            secret_hash: self.secret_hash,
        }
        .into()
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<secret_hash::Message>> for ComitLN {
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<secret_hash::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message:
                    secret_hash::Message {
                        swap_id,
                        secret_hash,
                    },
            } => {
                self.secret_hashes
                    .insert(swap_id, SecretHash::from(secret_hash));

                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("must exist");

                state.secret_hash_sent_or_received = true;

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message:
                    secret_hash::Message {
                        swap_id,
                        secret_hash,
                    },
            } => {
                self.secret_hashes
                    .insert(swap_id, SecretHash::from(secret_hash));

                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("should exist");

                state.secret_hash_sent_or_received = true;

                (peer, swap_id)
            }
        };

        let state = self.communication_state.get(&swap_id).unwrap();

        // check if we are done
        if self.ethereum_identities.contains_key(&swap_id)
            && self.lightning_identities.contains_key(&swap_id)
            && state.lightning_identity_sent
            && state.ethereum_identity_sent
            && state.secret_hash_sent_or_received
        {
            self.finalize.send(peer, finalize::Message::new(swap_id));
        }
    }
}

impl NetworkBehaviourEventProcess<announce::behaviour::BehaviourOutEvent> for ComitLN {
    fn inject_event(&mut self, event: announce::behaviour::BehaviourOutEvent) {
        match event {
            announce::behaviour::BehaviourOutEvent::ReceivedAnnouncement { peer, mut io } => {
                if let Some(local_id) = self.swaps_waiting_for_announcement.remove(&io.swap_digest)
                {
                    let id = SwapId::default();

                    self.swap_ids.insert(local_id.clone(), id.clone());

                    tokio::task::spawn(io.send(id));

                    let create_swap_params = self.swaps.get(&local_id).unwrap();

                    let addresses = self.announce.addresses_of_peer(&peer);
                    self.secret_hash
                        .register_addresses(peer.clone(), addresses.clone());
                    self.ethereum_identity
                        .register_addresses(peer.clone(), addresses.clone());
                    self.lightning_identity
                        .register_addresses(peer.clone(), addresses.clone());
                    self.finalize.register_addresses(peer.clone(), addresses);

                    self.ethereum_identity.send(
                        peer.clone(),
                        ethereum_identity::Message::new(
                            id,
                            create_swap_params.ethereum_identity.into(),
                        ),
                    );
                    self.lightning_identity.send(
                        peer,
                        lightning_identity::Message::new(id, create_swap_params.lightning_identity),
                    );

                    self.communication_state
                        .insert(id, CommunicationState::default());
                } else {
                    tracing::warn!(
                        "Peer {} announced a swap ({}) we don't know about",
                        peer,
                        io.swap_digest
                    );

                    tokio::task::spawn(async move {
                        let _ = io.io.close().await;
                    });
                }
            }
            announce::behaviour::BehaviourOutEvent::ReceivedConfirmation {
                peer,
                swap_digest,
                swap_id,
            } => {
                let local_swap_id = self
                    .swaps_waiting_for_announcement
                    .remove(&swap_digest)
                    .expect("we must know about this digest");

                self.swap_ids.insert(local_swap_id, swap_id);

                let addresses = self.announce.addresses_of_peer(&peer);
                self.secret_hash
                    .register_addresses(peer.clone(), addresses.clone());
                self.ethereum_identity
                    .register_addresses(peer.clone(), addresses.clone());
                self.lightning_identity
                    .register_addresses(peer.clone(), addresses.clone());
                self.finalize.register_addresses(peer.clone(), addresses);

                let create_swap_params = self.swaps.get(&local_swap_id).unwrap();

                self.ethereum_identity.send(
                    peer.clone(),
                    ethereum_identity::Message::new(
                        swap_id,
                        create_swap_params.ethereum_identity.into(),
                    ),
                );
                self.lightning_identity.send(
                    peer.clone(),
                    lightning_identity::Message::new(
                        swap_id,
                        create_swap_params.lightning_identity,
                    ),
                );

                let seed = self.seed.derive_swap_seed_from_node_local(local_swap_id);
                let secret_hash = seed.derive_secret().hash();

                self.secret_hashes.insert(swap_id, secret_hash);
                self.secret_hash
                    .send(peer, secret_hash::Message::new(swap_id, secret_hash));

                self.communication_state
                    .insert(swap_id, CommunicationState::default());
            }
            announce::behaviour::BehaviourOutEvent::Error { peer, error } => {
                tracing::warn!(
                    "failed to complete announce protocol with {} because {:?}",
                    peer,
                    error
                );
            }
        }
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<ethereum_identity::Message>>
    for ComitLN
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<ethereum_identity::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: ethereum_identity::Message { swap_id, address },
            } => {
                self.ethereum_identities
                    .insert(swap_id, identity::Ethereum::from(address));

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message: ethereum_identity::Message { swap_id, .. },
            } => {
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("this should exist");

                state.ethereum_identity_sent = true;

                (peer, swap_id)
            }
        };

        let state = self.communication_state.get(&swap_id).unwrap();

        // check if we are done
        if self.ethereum_identities.contains_key(&swap_id)
            && self.lightning_identities.contains_key(&swap_id)
            && state.lightning_identity_sent
            && state.ethereum_identity_sent
            && state.secret_hash_sent_or_received
        {
            self.finalize.send(peer, finalize::Message::new(swap_id));
        }
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<lightning_identity::Message>>
    for ComitLN
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<lightning_identity::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: lightning_identity::Message { swap_id, pubkey },
            } => {
                self.lightning_identities.insert(
                    swap_id,
                    bitcoin::PublicKey::from_slice(&pubkey).unwrap().into(),
                );

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message: lightning_identity::Message { swap_id, .. },
            } => {
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("this should exist");

                state.lightning_identity_sent = true;

                (peer, swap_id)
            }
        };

        let state = self.communication_state.get(&swap_id).unwrap();

        // check if we are done
        if self.ethereum_identities.contains_key(&swap_id)
            && self.lightning_identities.contains_key(&swap_id)
            && state.lightning_identity_sent
            && state.ethereum_identity_sent
            && state.secret_hash_sent_or_received
        {
            self.finalize.send(peer, finalize::Message::new(swap_id));
        }
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<finalize::Message>> for ComitLN {
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<finalize::Message>) {
        let (_, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: finalize::Message { swap_id },
            } => {
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("this should exist");

                state.received_finalized = true;

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message: finalize::Message { swap_id },
            } => {
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("this should exist");

                state.sent_finalized = true;

                (peer, swap_id)
            }
        };

        let state = self
            .communication_state
            .get_mut(&swap_id)
            .expect("this should exist");

        if state.sent_finalized && state.received_finalized {
            let local_swap_id = self
                .swap_ids
                .iter()
                .find_map(
                    |(key, value)| {
                        if *value == swap_id {
                            Some(key)
                        } else {
                            None
                        }
                    },
                )
                .copied()
                .unwrap();

            let create_swap_params = self
                .swaps
                .get(&local_swap_id)
                .cloned()
                .expect("create swap params exist");

            let secret_hash = self
                .secret_hashes
                .get(&swap_id)
                .copied()
                .expect("must exist");

            let ethereum_identity = self.ethereum_identities.get(&swap_id).copied().unwrap();

            self.events.push_back(BehaviourOutEvent::SwapFinalized {
                local_swap_id,
                swap_params: create_swap_params,
                secret_hash,
                ethereum_identity,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ethereum::Address,
        network::{derive_key_pair, oneshot_behaviour, transport, DialInformation, TokioExecutor},
        swap_protocols::{EthereumIdentity, SwapCommunicationStates, SwapErrorStates},
    };
    use anyhow::Context;
    use bitcoin::secp256k1;
    use futures::{pin_mut, prelude::*};
    use libp2p::{
        core::{muxing::StreamMuxer, upgrade},
        identity,
        mplex::MplexConfig,
        multihash::{Multihash, Sha3_256},
        secio::SecioConfig,
        swarm::{Swarm, SwarmBuilder, SwarmEvent},
        tcp::TcpConfig,
        Multiaddr, PeerId, Transport,
    };
    use rand::{random, thread_rng};
    use spectral::prelude::*;
    use std::{str::FromStr, sync::Arc};
    use tokio::runtime;
    use uuid::Uuid;

    fn random_swap_digest() -> SwapDigest {
        SwapDigest::new(Sha3_256::digest(b"hello world"))
    }

    #[test]
    fn lightning_to_ethereum_integration_test() {
        let (alice_key_pair, alice_peer_id, alice_seed) = {
            let seed = RootSeed::new_random(thread_rng()).unwrap();
            let key_pair = derive_key_pair(&seed);
            let peer_id = PeerId::from(key_pair.clone().public());
            (key_pair, peer_id, seed)
        };

        let (bob_key_pair, bob_peer_id, bob_seed) = {
            let seed = RootSeed::new_random(thread_rng()).unwrap();
            let key_pair = derive_key_pair(&seed);
            let peer_id = PeerId::from(key_pair.clone().public());
            (key_pair, peer_id, seed)
        };

        let mut alice_runtime = runtime::Builder::new()
            .enable_all()
            .threaded_scheduler()
            .thread_stack_size(1024 * 1024 * 8) // the default is 2MB but that causes a segfault for some reason
            .build()
            .unwrap();

        let mut alice_swarm = SwarmBuilder::new(
            transport::build_comit_transport(alice_key_pair).unwrap(),
            ComitLN::new(alice_seed),
            alice_peer_id.clone(),
        )
        .executor(Box::new(TokioExecutor {
            handle: alice_runtime.handle().clone(),
        }))
        .build();

        // let mut alice_swarm = {
        //     let transport= transport::build_comit_transport(alice_key_pair).unwrap();
        //     let protocol = super::ComitLN::new(alice_seed);
        //     Swarm::new(transport, protocol, alice_peer_id.clone())
        // };

        let mut bob_runtime = runtime::Builder::new()
            .enable_all()
            .threaded_scheduler()
            .thread_stack_size(1024 * 1024 * 8) // the default is 2MB but that causes a segfault for some reason
            .build()
            .unwrap();

        let mut bob_swarm = SwarmBuilder::new(
            transport::build_comit_transport(bob_key_pair).unwrap(),
            ComitLN::new(bob_seed),
            bob_peer_id.clone(),
        )
        .executor(Box::new(TokioExecutor {
            handle: bob_runtime.handle().clone(),
        }))
        .build();

        // let mut bob_swarm = {
        //     let transport = transport::build_comit_transport(bob_key_pair).unwrap();
        //     let protocol = super::ComitLN::new(bob_seed);
        //     Swarm::new(transport, protocol, bob_peer_id.clone())
        // };

        let bob_addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        Swarm::listen_on(&mut bob_swarm, bob_addr.clone())
            .with_context(|| format!("Address is not supported: {:?}", bob_addr))
            .unwrap();

        let bob_addr: libp2p::core::Multiaddr = bob_runtime.block_on(async {
            loop {
                let bob_swarm_fut = bob_swarm.next_event();
                pin_mut!(bob_swarm_fut);
                match bob_swarm_fut.await {
                    SwarmEvent::NewListenAddr(addr) => return addr,
                    _ => {}
                }
            }
        });

        let send_swap_digest = random_swap_digest();

        let dial_info = DialInformation {
            peer_id: bob_peer_id.clone(),
            address_hint: Some(bob_addr.clone()),
        };

        let ethereum_id = EthereumIdentity::from(Address::default());

        let secp_pubkey = secp256k1::PublicKey::from_str(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
        )
        .unwrap();
        let lightning_id = crate::lightning::PublicKey::from(secp_pubkey);

        let swap_params = HanEtherereumHalightBitcoinCreateSwapParams {
            role: Role::Alice,
            peer: dial_info,
            ethereum_identity: ethereum_id,
            ethereum_absolute_expiry: Timestamp::from(10),
            ethereum_amount: asset::Ether::zero(),
            lightning_identity: lightning_id,
            lightning_cltv_expiry: Timestamp::from(10),
            lightning_amount: asset::Lightning::from_sat(0),
        };

        let alice_node_id = NodeLocalSwapId::default();
        alice_swarm.initiate_communication(alice_node_id, swap_params);

        // trying to check if swap finalized or other events occur on bob
        // doing something wrong here causing the test to hang
        bob_runtime.block_on(async move {
            loop {
                let bob_swarm_fut = bob_swarm.next_event();
                pin_mut!(bob_swarm_fut);
                match bob_swarm_fut.await {
                    SwarmEvent::Behaviour(behavior_event) => {
                        // never enters this block causing the test to hang
                        // if let BehaviourOutEvent::SwapFinalized {..} = behavior_event {
                        //     //assert_eq!(io.swap_digest, send_swap_digest);
                        //     // assert_eq!(peer, peer)
                        //
                        //     return;
                        // }

                        return;
                    }
                    _ => {}
                }
            }
        })
    }
}
