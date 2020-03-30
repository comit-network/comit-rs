use crate::{
    asset,
    asset::ethereum::TryFromWei,
    btsieve::ethereum::{Cache, Web3Connector},
    htlc_location,
    http_api::routes::index::Body,
    identity,
    lnd::{LndConnectorAsRecipient, LndConnectorAsSender, LndConnectorParams},
    network::{
        oneshot_behaviour,
        protocols::{
            announce,
            announce::{
                behaviour::{Announce, BehaviourOutEvent},
                SwapDigest,
            },
            ethereum_identity, finalize, lightning_identity, secret_hash,
        },
    },
    seed::{DeriveSwapSeedFromNodeLocal, RootSeed},
    swap_protocols::{
        halight::{self, InvoiceStates},
        han, ledger,
        ledger::{ethereum::ChainId, lightning, Ethereum},
        rfc003::{create_swap::HtlcParams, DeriveSecret, SecretHash},
        LedgerStates, NodeLocalSwapId, Role, SwapId,
    },
    timestamp::Timestamp,
    transaction,
};
use chrono::Utc;
use futures::AsyncWriteExt;
use libp2p::{multihash, swarm::NetworkBehaviourEventProcess, NetworkBehaviour};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tracing_futures::Instrument;

#[derive(NetworkBehaviour, Debug)]
pub struct ComitLN {
    announce: Announce,
    secret_hash: oneshot_behaviour::Behaviour<secret_hash::Message>,
    ethereum_identity: oneshot_behaviour::Behaviour<ethereum_identity::Message>,
    lightning_identity: oneshot_behaviour::Behaviour<lightning_identity::Message>,
    finalize: oneshot_behaviour::Behaviour<finalize::Message>,

    // TODO: Quick and dirty state tracking that doesn't scale
    // refactor this to something more elegant that covers all combinations
    #[behaviour(ignore)]
    swaps_waiting_for_announcement: HashMap<SwapDigest, NodeLocalSwapId>,
    #[behaviour(ignore)]
    swaps: HashMap<NodeLocalSwapId, Body>,
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
    lnd_connector_as_sender: Arc<LndConnectorAsSender>,
    #[behaviour(ignore)]
    lnd_connector_as_recipient: Arc<LndConnectorAsRecipient>,

    // FIXME: Ethereum stuff only (han-halight)
    #[behaviour(ignore)]
    ethereum_connector: Arc<Cache<Web3Connector>>,
    #[behaviour(ignore)]
    ethereum_ledger_state: Arc<LedgerStates>,

    // FIXME: Is this ok here?
    #[behaviour(ignore)]
    pub seed: RootSeed,
}

#[derive(Debug, Default)]
struct CommunicationState {
    ethereum_identity_sent: bool,
    lightning_identity_sent: bool,
    received_finalized: bool,
    sent_finalized: bool,
    // TODO: this is "sent" for Alice and "received" for Bob
    // needs to be modelled better, together with all of this state tracking
    secret_hash_sent_or_received: bool,
}

impl ComitLN {
    pub fn new(
        lnd_connector_params: LndConnectorParams,
        ethereum_connector: Arc<Cache<Web3Connector>>,
        ethereum_ledger_state: Arc<LedgerStates>,
        seed: RootSeed,
    ) -> Self {
        ComitLN {
            announce: Default::default(),
            secret_hash: Default::default(),
            ethereum_identity: Default::default(),
            lightning_identity: Default::default(),
            finalize: Default::default(),
            swaps_waiting_for_announcement: Default::default(),
            swaps: Default::default(),
            swap_ids: Default::default(),
            ethereum_identities: Default::default(),
            lightning_identities: Default::default(),
            communication_state: Default::default(),
            secret_hashes: Default::default(),
            lnd_connector_as_sender: Arc::new(lnd_connector_params.clone().into()),
            lnd_connector_as_recipient: Arc::new(lnd_connector_params.into()),
            ethereum_connector,
            ethereum_ledger_state,
            seed,
        }
    }

    pub fn initiate_communication(&mut self, id: NodeLocalSwapId, body: Body) {
        let digest = SwapDigest {
            inner: multihash::encode(
                multihash::Hash::SHA2256,
                b"TODO REPLACE ME WITH THE ACTUAL SWAP DIGEST",
            )
            .to_owned()
            .unwrap(),
        };

        self.swaps.insert(id, body.clone());

        match body.role() {
            Role::Alice => {
                self.announce
                    .start_announce_protocol(digest.clone(), body.peer);

                if self.swaps_waiting_for_announcement.contains_key(&digest) {
                    // To fix this panic, we should either pass the local swap id to the
                    // announce behaviour or get a unique token from the behaviour that
                    // we can use to track the progress of the announcement
                    panic!("cannot send two swaps with the same digest at the same time!")
                }

                self.swaps_waiting_for_announcement.insert(digest, id);
            }
            Role::Bob => {
                self.swaps_waiting_for_announcement.insert(digest, id);
            }
        }
    }

    pub fn get_finalized_swap(&self, id: SwapId) -> Option<FinalizedSwap> {
        let local_id = NodeLocalSwapId(id.0);

        let body = match self.swaps.get(&local_id) {
            Some(body) => body,
            None => return None,
        };

        let alpha_ledger_redeem_identity = match body.role() {
            Role::Alice => self.ethereum_identities.get(&id).copied().unwrap(),
            Role::Bob => body.alpha.identity.parse().unwrap(),
        };
        let alpha_ledger_refund_identity = match body.role() {
            Role::Alice => body.alpha.identity.parse().unwrap(),
            Role::Bob => self.ethereum_identities.get(&id).copied().unwrap(),
        };
        let beta_ledger_redeem_identity = match body.role() {
            Role::Alice => self.lightning_identities.get(&id).copied().unwrap(),
            Role::Bob => body.beta.identity.parse().unwrap(),
        };
        let beta_ledger_refund_identity = match body.role() {
            Role::Alice => body.beta.identity.parse().unwrap(),
            Role::Bob => self.lightning_identities.get(&id).copied().unwrap(),
        };

        Some(FinalizedSwap {
            alpha_ledger: Ethereum::new(ChainId::regtest()), // TODO: don't hardcode these
            beta_ledger: lightning::Regtest,                 // TODO: don't hardcode these
            alpha_asset: asset::Ether::try_from_wei(body.alpha.amount.as_str()).unwrap(), /* TODO: don't unwrap */
            beta_asset: asset::Lightning::from_sat(body.beta.amount.parse().unwrap()), /* TODO: don't unwrap */
            alpha_ledger_redeem_identity,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            beta_ledger_refund_identity,
            alpha_expiry: body.alpha.absolute_expiry,
            beta_expiry: body.beta.cltv_expiry, // TODO: is this correct?
            secret_hash: self.secret_hashes.get(&id).copied().unwrap(),
            role: body.role(),
        })
    }
}

// TODO: this is just a temporary struct and should likely be replaced with
// something more generic Also reconsider whether we need to pass everything
// back up the call chain TODO: is there a better name for this?
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
    pub alpha_expiry: u32,
    pub beta_expiry: u32,
    pub secret_hash: SecretHash,
    pub role: Role,
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<secret_hash::Message>> for ComitLN {
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<secret_hash::Message>) {
        let (peer, swap_id) = match event {
            // TODO: Refactor this, Received/Sent is the same.
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
    fn inject_event(&mut self, event: BehaviourOutEvent) {
        match event {
            // TODO: rename this to something better, like `ReceivedAnnouncement`
            BehaviourOutEvent::AwaitingConfirmation { peer, mut io } => {
                if let Some(local_id) = self.swaps_waiting_for_announcement.remove(&io.swap_digest)
                {
                    let id = SwapId::default();

                    self.swap_ids.insert(local_id.clone(), id.clone());

                    // TODO: don't use global spawn function?
                    tokio::task::spawn(io.send(id));

                    let body = self.swaps.get(&local_id).unwrap();

                    let address = identity::Ethereum::from_str(&body.alpha.identity).unwrap();
                    let pubkey = identity::Lightning::from_str(&body.beta.identity).unwrap();

                    self.ethereum_identity
                        .send(peer.clone(), ethereum_identity::Message::new(id, address));
                    self.lightning_identity
                        .send(peer, lightning_identity::Message::new(id, pubkey));

                    self.communication_state
                        .insert(id, CommunicationState::default());
                } else {
                    // TODO: if digest is not present, save it to some other kind of hashmap/hashset
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
            BehaviourOutEvent::ReceivedConfirmation {
                peer,
                swap_digest,
                swap_id,
            } => {
                let local_swap_id = self
                    .swaps_waiting_for_announcement
                    .remove(&swap_digest)
                    .expect("we must know about this digest");

                self.swap_ids.insert(local_swap_id, swap_id);

                let body = self.swaps.get(&local_swap_id).unwrap();

                let address = identity::Ethereum::from_str(&body.alpha.identity).unwrap();
                let pubkey = identity::Lightning::from_str(&body.beta.identity).unwrap();

                self.ethereum_identity.send(
                    peer.clone(),
                    ethereum_identity::Message::new(swap_id, address),
                );
                self.lightning_identity.send(
                    peer.clone(),
                    lightning_identity::Message::new(swap_id, pubkey),
                );

                let seed = self.seed.derive_swap_seed_from_node_local(local_swap_id);
                let secret_hash = seed.derive_secret().hash();

                self.secret_hashes.insert(swap_id, secret_hash);
                self.secret_hash
                    .send(peer, secret_hash::Message::new(swap_id, secret_hash));

                self.communication_state
                    .insert(swap_id, CommunicationState::default());
            }
            BehaviourOutEvent::Error { .. } => {
                // TODO: How do we know which swap failed ?!
                unimplemented!("we failed to complete the announce protocol")
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

            let body = self.swaps.get(&local_swap_id).cloned().expect("must exist");

            let secret_hash = self
                .secret_hashes
                .get(&swap_id)
                .copied()
                .expect("must exist");
            let halight_asset =
                asset::Lightning::from_sat(u64::from_str(&body.beta.amount).unwrap());

            let invoice_states = Arc::new(InvoiceStates::default());

            if body.role() == Role::Alice {
                tokio::task::spawn({
                    let lnd_connector = (*self.lnd_connector_as_recipient)
                        .clone()
                        // TODO: Panicking now may not be the best.
                        // It would be great to do this part when REST API call is received
                        .read_certificate()
                        .expect("Failure reading tls certificate");
                    let bob_ln_identity = self.lightning_identities.get(&swap_id).copied().unwrap();
                    let alice_ln_identity =
                        identity::Lightning::from_str(&body.beta.identity).unwrap();
                    let expiry = Timestamp::from(body.beta.cltv_expiry);

                    async move {
                        halight::create_watcher(
                            &lnd_connector,
                            invoice_states,
                            swap_id,
                            halight::Params {
                                asset: halight_asset,
                                ledger: ledger::lightning::Regtest,
                                redeem_identity: alice_ln_identity,
                                refund_identity: bob_ln_identity,
                                expiry,
                                secret_hash,
                            },
                            Utc::now().naive_local(), // TODO don't create this here
                        )
                        .instrument(tracing::info_span!("halight"))
                        .await;
                    }
                });
            } else {
                // This is Bob
                tokio::task::spawn({
                    let lnd_connector = (*self.lnd_connector_as_sender)
                        .clone()
                        // TODO: Panicking now may not be the best.
                        // It would be great to do this part when REST API call is received
                        .read_certificate()
                        .expect("Failure reading tls certificate");
                    let alice_ln_identity =
                        self.lightning_identities.get(&swap_id).copied().unwrap();
                    let bob_ln_identity =
                        identity::Lightning::from_str(&body.beta.identity).unwrap();
                    let expiry = Timestamp::from(body.beta.cltv_expiry);

                    async move {
                        halight::create_watcher(
                            &lnd_connector,
                            invoice_states,
                            swap_id,
                            halight::Params {
                                asset: halight_asset,
                                ledger: ledger::lightning::Regtest,
                                redeem_identity: alice_ln_identity,
                                refund_identity: bob_ln_identity,
                                expiry,
                                secret_hash,
                            },
                            Utc::now().naive_local(), // TODO don't create this here
                        )
                        .instrument(tracing::info_span!("halight"))
                        .await;
                    }
                });
            }

            if body.role() == Role::Alice {
                tokio::task::spawn({
                    let connector = self.ethereum_connector.clone();
                    let alice_ethereum_identity =
                        identity::Ethereum::from_str(&body.alpha.identity).unwrap();
                    let bob_ethereum_identity =
                        self.ethereum_identities.get(&swap_id).copied().unwrap();

                    let asset = asset::Ether::from_wei_dec_str(&body.alpha.amount).unwrap();
                    let ledger = ledger::Ethereum::default(); // FIXME: get this from somewhere
                    let expiry = Timestamp::from(body.alpha.absolute_expiry);
                    let secret_hash = self
                        .secret_hashes
                        .get(&swap_id)
                        .copied()
                        .expect("must exist");

                    let htlc_params = HtlcParams {
                        asset,
                        ledger,
                        redeem_identity: bob_ethereum_identity,
                        refund_identity: alice_ethereum_identity,
                        expiry,
                        secret_hash,
                    };

                    let ethereum_ledger_state = self.ethereum_ledger_state.clone();

                    async move {
                        han::create_watcher::<
                            _,
                            _,
                            _,
                            _,
                            htlc_location::Ethereum,
                            _,
                            transaction::Ethereum,
                        >(
                            connector.as_ref(),
                            ethereum_ledger_state,
                            swap_id,
                            htlc_params,
                            Utc::now().naive_local(), // TODO don't create this here
                        )
                        .instrument(tracing::info_span!("han"))
                        .await
                    }
                });
            } else {
                // This is Bob
                tokio::task::spawn({
                    let connector = self.ethereum_connector.clone();
                    let alice_ethereum_identity =
                        self.ethereum_identities.get(&swap_id).copied().unwrap();
                    let bob_ethereum_identity =
                        identity::Ethereum::from_str(&body.alpha.identity).unwrap();

                    let asset = asset::Ether::from_wei_dec_str(&body.alpha.amount).unwrap();
                    let ledger = ledger::Ethereum::default(); // FIXME: get this from somewhere
                    let expiry = Timestamp::from(body.alpha.absolute_expiry);
                    let secret_hash = self.secret_hashes.get(&swap_id).copied().unwrap();

                    let htlc_params = HtlcParams {
                        asset,
                        ledger,
                        redeem_identity: bob_ethereum_identity,
                        refund_identity: alice_ethereum_identity,
                        expiry,
                        secret_hash,
                    };

                    let ethereum_ledger_state = self.ethereum_ledger_state.clone();

                    async move {
                        han::create_watcher::<
                            _,
                            _,
                            _,
                            _,
                            htlc_location::Ethereum,
                            _,
                            transaction::Ethereum,
                        >(
                            connector.as_ref(),
                            ethereum_ledger_state,
                            swap_id,
                            htlc_params,
                            Utc::now().naive_local(), // TODO don't create this here
                        )
                        .instrument(tracing::info_span!("han"))
                        .await
                    }
                });
            }
        }
    }
}
