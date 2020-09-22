use crate::{
    asset, ethereum::ChainId, hbit, herc20, identity, ledger, Role, SecretHash, Timestamp,
};
use anyhow::Result;
use futures::prelude::*;
use libp2p::{
    core::upgrade,
    request_response::{
        ProtocolName, ProtocolSupport, RequestResponse, RequestResponseCodec,
        RequestResponseConfig, RequestResponseEvent, RequestResponseMessage,
    },
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    io,
    marker::PhantomData,
    task::{Context, Poll},
};

#[derive(Clone, Copy, Debug, thiserror::Error)]
#[error("Already have role dependent parameters for this set of common parameters")]
pub struct AlreadyHaveRoleParams;

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum BehaviourOutEvent<C> {
    // TODO: Split this into two events, one for HbitHerc20 and one for Herc20Hbit
    ExecutableSwap(ExecutableSwap<C>),
    AlreadyHaveRoleParams {
        peer: PeerId,
        have: RoleDependentParams,
        received: RoleDependentParams,
    },
}

#[derive(Clone, Debug)]
pub struct ExecutableSwap<C> {
    pub our_role: Role,
    pub herc20: herc20::Params,
    pub hbit: hbit::Params,
    pub swap_protocol: SwapProtocol,
    pub peer_id: PeerId,
    pub context: C,
}

impl<C> BehaviourOutEvent<C> {
    fn new_executable_swap(
        our_role: Role,
        common: CommonParams,
        alice: &AliceParams,
        bob: &BobParams,
        swap_protocol: SwapProtocol,
        peer_id: PeerId,
        context: C,
    ) -> Self {
        match swap_protocol {
            SwapProtocol::HbitHerc20 => BehaviourOutEvent::ExecutableSwap(ExecutableSwap {
                our_role,
                herc20: herc20::Params {
                    asset: common.erc20,
                    redeem_identity: alice.ethereum_identity,
                    refund_identity: bob.ethereum_identity,
                    expiry: Timestamp::from(common.ethereum_absolute_expiry),
                    secret_hash: alice.secret_hash,
                    chain_id: common.ethereum_chain_id,
                },
                hbit: hbit::Params {
                    network: common.bitcoin_network,
                    asset: common.bitcoin,
                    redeem_identity: bob.bitcoin_identity,
                    refund_identity: alice.bitcoin_identity,
                    expiry: Timestamp::from(common.bitcoin_absolute_expiry),
                    secret_hash: alice.secret_hash,
                },
                swap_protocol,
                peer_id,
                context,
            }),
            SwapProtocol::Herc20Hbit => BehaviourOutEvent::ExecutableSwap(ExecutableSwap {
                our_role,
                herc20: herc20::Params {
                    asset: common.erc20,
                    redeem_identity: bob.ethereum_identity,
                    refund_identity: alice.ethereum_identity,
                    expiry: Timestamp::from(common.ethereum_absolute_expiry),
                    secret_hash: alice.secret_hash,
                    chain_id: common.ethereum_chain_id,
                },
                hbit: hbit::Params {
                    network: common.bitcoin_network,
                    asset: common.bitcoin,
                    redeem_identity: alice.bitcoin_identity,
                    refund_identity: bob.bitcoin_identity,
                    expiry: Timestamp::from(common.bitcoin_absolute_expiry),
                    secret_hash: alice.secret_hash,
                },
                swap_protocol,
                peer_id,
                context,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwapProtocol {
    HbitHerc20,
    Herc20Hbit,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent<C>", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct SetupSwap<C: Clone + Send + 'static> {
    hbit_herc20: RequestResponse<Codec<HbitHerc20Protocol>>,
    herc20_hbit: RequestResponse<Codec<Herc20HbitProtocol>>,
    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent<C>>,
    #[behaviour(ignore)]
    swap_data: HashMap<CommonParams, RoleDependentParams>,
    #[behaviour(ignore)]
    context: HashMap<CommonParams, C>,
}

impl<C: Clone + Send + 'static> Default for SetupSwap<C> {
    fn default() -> Self {
        SetupSwap {
            hbit_herc20: RequestResponse::new(
                Codec::default(),
                vec![(HbitHerc20Protocol, ProtocolSupport::Full)],
                RequestResponseConfig::default(),
            ),
            herc20_hbit: RequestResponse::new(
                Codec::default(),
                vec![(Herc20HbitProtocol, ProtocolSupport::Full)],
                RequestResponseConfig::default(),
            ),
            events: Default::default(),
            swap_data: Default::default(),
            context: Default::default(),
        }
    }
}

impl<C: Clone + Send + 'static> SetupSwap<C> {
    pub fn send(
        &mut self,
        to: &PeerId,
        to_send: RoleDependentParams,
        common: CommonParams,
        swap_protocol: SwapProtocol,
        context: C,
    ) -> Result<()> {
        match (self.swap_data.get(&common), &to_send) {
            (Some(RoleDependentParams::Alice(alice)), RoleDependentParams::Bob(bob)) => {
                self.events
                    .push_back(BehaviourOutEvent::new_executable_swap(
                        Role::Bob,
                        common.clone(),
                        alice,
                        &bob,
                        swap_protocol,
                        to.clone(),
                        context,
                    ));
            }
            (Some(RoleDependentParams::Bob(bob)), RoleDependentParams::Alice(alice)) => {
                self.events
                    .push_back(BehaviourOutEvent::new_executable_swap(
                        Role::Alice,
                        common.clone(),
                        &alice,
                        bob,
                        swap_protocol,
                        to.clone(),
                        context,
                    ));
            }
            (None, role) => {
                self.swap_data.insert(common.clone(), *role);
                self.context.insert(common.clone(), context);
            }
            (Some(_), _) => return Err(anyhow::Error::from(AlreadyHaveRoleParams)),
        }

        tracing::info!("Setting up swap with {}", to);

        let _ = match swap_protocol {
            SwapProtocol::Herc20Hbit => self
                .herc20_hbit
                .send_request(to, to_send.to_message(common)),
            SwapProtocol::HbitHerc20 => self
                .hbit_herc20
                .send_request(to, to_send.to_message(common)),
        };
        Ok(())
    }

    fn alice_receive_hbit_herc20(&mut self, from: PeerId, common: CommonParams, bob: BobParams) {
        self.alice_receive(from, common, bob, SwapProtocol::HbitHerc20);
    }

    fn alice_receive_herc20_hbit(&mut self, from: PeerId, common: CommonParams, bob: BobParams) {
        self.alice_receive(from, common, bob, SwapProtocol::Herc20Hbit);
    }

    fn bob_receive_hbit_herc20(&mut self, from: PeerId, common: CommonParams, alice: AliceParams) {
        self.bob_receive(from, common, alice, SwapProtocol::HbitHerc20);
    }

    fn bob_receive_herc20_hbit(&mut self, from: PeerId, common: CommonParams, alice: AliceParams) {
        self.bob_receive(from, common, alice, SwapProtocol::Herc20Hbit);
    }
    fn alice_receive(
        &mut self,
        from: PeerId,
        common: CommonParams,
        bob: BobParams,
        swap_protocol: SwapProtocol,
    ) {
        match self.swap_data.get(&common) {
            Some(RoleDependentParams::Alice(alice)) => {
                // todo: remove unwrap
                let context = self.context.get(&common).cloned().unwrap();
                self.events
                    .push_back(BehaviourOutEvent::new_executable_swap(
                        Role::Alice,
                        common,
                        alice,
                        &bob,
                        swap_protocol,
                        from,
                        context,
                    ));
            }
            Some(RoleDependentParams::Bob(have)) => {
                self.events
                    .push_back(BehaviourOutEvent::AlreadyHaveRoleParams {
                        peer: from,
                        have: RoleDependentParams::Bob(*have),
                        received: RoleDependentParams::Bob(bob),
                    });
            }
            None => {
                self.swap_data
                    .insert(common.clone(), RoleDependentParams::Bob(bob));
            }
        }
    }

    fn bob_receive(
        &mut self,
        from: PeerId,
        common: CommonParams,
        alice: AliceParams,
        swap_protocol: SwapProtocol,
    ) {
        match self.swap_data.get(&common) {
            Some(RoleDependentParams::Alice(have)) => {
                self.events
                    .push_back(BehaviourOutEvent::AlreadyHaveRoleParams {
                        peer: from,
                        have: RoleDependentParams::Alice(*have),
                        received: RoleDependentParams::Alice(alice),
                    });
            }
            Some(RoleDependentParams::Bob(bob)) => {
                // todo: remove unwrap
                let context = self.context.get(&common).cloned().unwrap();
                self.events
                    .push_back(BehaviourOutEvent::new_executable_swap(
                        Role::Bob,
                        common,
                        &alice,
                        bob,
                        swap_protocol,
                        from,
                        context,
                    ))
            }
            None => {
                self.swap_data
                    .insert(common.clone(), RoleDependentParams::Alice(alice));
            }
        }
    }

    fn poll<InEvent>(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<InEvent, BehaviourOutEvent<C>>> {
        if let Some(event) = self.events.pop_front() {
            if let BehaviourOutEvent::ExecutableSwap(swap) = &event {
                tracing::info!("Successfully set up swap with {}", swap.peer_id);
            }

            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}

impl<C: Clone + Send + 'static>
    NetworkBehaviourEventProcess<RequestResponseEvent<Message<HbitHerc20Protocol>, ()>>
    for SetupSwap<C>
{
    fn inject_event(&mut self, event: RequestResponseEvent<Message<HbitHerc20Protocol>, ()>) {
        match event {
            RequestResponseEvent::Message {
                peer,
                message:
                    RequestResponseMessage::Request {
                        request: message, ..
                    },
            } => match message {
                Message::Alice { alice, common, .. } => {
                    self.bob_receive_hbit_herc20(peer, common, alice)
                }
                Message::Bob { bob, common, .. } => {
                    self.alice_receive_hbit_herc20(peer, common, bob)
                }
            },
            RequestResponseEvent::OutboundFailure { error, .. } => {
                tracing::warn!("outbound failure: {:?}", error);
            }
            RequestResponseEvent::InboundFailure { error, .. } => {
                tracing::warn!("inbound failure: {:?}", error);
            }
            _ => {}
        }
    }
}

impl<C: Clone + Send + 'static>
    NetworkBehaviourEventProcess<RequestResponseEvent<Message<Herc20HbitProtocol>, ()>>
    for SetupSwap<C>
{
    fn inject_event(&mut self, event: RequestResponseEvent<Message<Herc20HbitProtocol>, ()>) {
        match event {
            RequestResponseEvent::Message {
                peer,
                message:
                    RequestResponseMessage::Request {
                        request: message, ..
                    },
            } => match message {
                Message::Alice { alice, common, .. } => {
                    self.bob_receive_herc20_hbit(peer, common, alice)
                }
                Message::Bob { bob, common, .. } => {
                    self.alice_receive_herc20_hbit(peer, common, bob)
                }
            },
            RequestResponseEvent::OutboundFailure { error, .. } => {
                tracing::warn!("outbound failure: {:?}", error);
            }
            RequestResponseEvent::InboundFailure { error, .. } => {
                tracing::warn!("inbound failure: {:?}", error);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HbitHerc20Protocol;

impl ProtocolName for HbitHerc20Protocol {
    fn protocol_name(&self) -> &[u8] {
        b"/comit/setup-swap/hbit-herc20/1.0.0"
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Herc20HbitProtocol;

impl ProtocolName for Herc20HbitProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/comit/setup-swap/herc20-hbit/1.0.0"
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Codec<U: ProtocolName + Send + Clone>(PhantomData<U>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CommonParams {
    pub erc20: asset::Erc20,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub bitcoin: asset::Bitcoin,
    pub ethereum_absolute_expiry: u32,
    pub bitcoin_absolute_expiry: u32,
    pub ethereum_chain_id: ChainId,
    pub bitcoin_network: ledger::Bitcoin,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct AliceParams {
    pub ethereum_identity: identity::Ethereum,
    pub bitcoin_identity: identity::Bitcoin,
    pub secret_hash: SecretHash,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BobParams {
    pub ethereum_identity: identity::Ethereum,
    pub bitcoin_identity: identity::Bitcoin,
}

#[derive(Debug, Copy, Clone)]
pub enum RoleDependentParams {
    Alice(AliceParams),
    Bob(BobParams),
}

impl RoleDependentParams {
    fn to_message<U>(self, common: CommonParams) -> Message<U> {
        match self {
            RoleDependentParams::Alice(alice) => Message::Alice {
                _marker: PhantomData,
                common,
                alice,
            },
            RoleDependentParams::Bob(bob) => Message::Bob {
                _marker: PhantomData,
                common,
                bob,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message<U> {
    Alice {
        _marker: PhantomData<U>,
        common: CommonParams,
        alice: AliceParams,
    },
    Bob {
        _marker: PhantomData<U>,
        common: CommonParams,
        bob: BobParams,
    },
}

#[async_trait::async_trait]
impl<U> RequestResponseCodec for Codec<U>
where
    U: ProtocolName + Sync + Send + Clone,
{
    type Protocol = U;
    type Request = Message<U>;
    type Response = ();

    /// Reads a take order request from the given I/O stream.
    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let message = upgrade::read_one(io, 1024)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut de = serde_json::Deserializer::from_slice(&message);
        let req = Message::deserialize(&mut de)?;

        Ok(req)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        _io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        Ok(())
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = serde_json::to_vec(&req)?;
        upgrade::write_one(io, &bytes).await?;

        Ok(())
    }

    #[allow(clippy::unit_arg)]
    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        _io: &mut T,
        _res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network::test::{await_events_or_timeout, connect, new_swarm},
        Secret,
    };
    use bitcoin::secp256k1;
    use std::{future::Future, str::FromStr};

    #[tokio::test]
    async fn given_bob_sends_when_alice_sends_one_then_swap_is_confirmed() {
        let (mut alice_swarm, _, alice_id) = new_swarm(|_, _| SetupSwap::default());
        let (mut bob_swarm, _, bob_id) = new_swarm(|_, _| SetupSwap::default());
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let ethereum_identity = identity::Ethereum::random();
        let bitcoin_identity = identity::Bitcoin::from(
            secp256k1::PublicKey::from_str(
                "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
            )
            .unwrap(),
        );
        let secret_hash = SecretHash::new(
            Secret::from_str("68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4c66")
                .expect("could not convert string to secret"),
        );

        let common = CommonParams {
            erc20: asset::Erc20::new(identity::Ethereum::random(), asset::Erc20Quantity::zero()),
            bitcoin: asset::Bitcoin::from_sat(0),
            ethereum_absolute_expiry: 0,
            bitcoin_absolute_expiry: 0,
            ethereum_chain_id: ChainId::GETH_DEV,
            bitcoin_network: ledger::Bitcoin::Regtest,
        };

        let alice_context = 1;
        let bob_context = 2;

        bob_swarm
            .send(
                &alice_id,
                RoleDependentParams::Bob(BobParams {
                    ethereum_identity,
                    bitcoin_identity,
                }),
                common.clone(),
                SwapProtocol::HbitHerc20,
                bob_context,
            )
            .expect("bob failed to send");
        alice_swarm
            .send(
                &bob_id,
                RoleDependentParams::Alice(AliceParams {
                    ethereum_identity,
                    bitcoin_identity,
                    secret_hash,
                }),
                common,
                SwapProtocol::HbitHerc20,
                alice_context,
            )
            .expect("alice failed to send");

        assert_both_confirmed(
            alice_swarm.next(),
            bob_swarm.next(),
            alice_context,
            bob_context,
        )
        .await;
    }

    async fn assert_both_confirmed<C: PartialEq + Debug>(
        alice_event: impl Future<Output = BehaviourOutEvent<C>>,
        bob_event: impl Future<Output = BehaviourOutEvent<C>>,
        expected_alice_context: C,
        expected_bob_context: C,
    ) {
        match await_events_or_timeout(alice_event, bob_event).await {
            (
                BehaviourOutEvent::ExecutableSwap(ExecutableSwap {
                    our_role: alice_role, hbit: alice_hbit,
                    herc20: alice_herc20, swap_protocol: alice_swap_protocol, context: alice_context, .. }),
                BehaviourOutEvent::ExecutableSwap(ExecutableSwap {
                    our_role: bob_role,  herc20: bob_herc20, hbit: bob_hbit, swap_protocol: bob_swap_protocol, context: bob_context, ..}),
            ) => {
                assert_ne!(alice_role, bob_role);
                assert_eq!(alice_hbit, bob_hbit);
                assert_eq!(alice_herc20, bob_herc20);
                assert_eq!(alice_swap_protocol, bob_swap_protocol);
                assert_eq!(expected_alice_context, alice_context);
                assert_eq!(expected_bob_context, bob_context);

            }
            (alice_event, bob_event) => panic!("expected both parties to confirm the swap but alice emitted {:?} and bob emitted {:?}", alice_event, bob_event),
        }
    }
}
