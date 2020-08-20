use crate::{asset, ethereum::ChainId, hbit, herc20, identity, Role, SecretHash, Timestamp};
use bitcoin::{hashes::core::fmt::Formatter, Network};
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
    fmt::{Debug, Display},
    io,
    marker::PhantomData,
    task::{Context, Poll},
};

/// Event emitted by the `Comit` behaviour.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum BehaviourOutEvent {
    Herc20Hbit {
        alpha: herc20::Params,
        beta: hbit::Params,
    },
    HbitHerc20 {
        alpha: hbit::Params,
        beta: herc20::Params,
    },
    AlreadyHaveRoleParams {
        peer: PeerId,
        have: RoleDependentParams,
        received: RoleDependentParams,
    },
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("peer {peer} also wants to be {role} for swap {id}")]
pub struct RoleMistmatch {
    peer: PeerId,
    role: Role,
    id: CommonParams,
}

impl BehaviourOutEvent {
    fn new(
        common: CommonParams,
        alice: &AliceParams,
        bob: &BobParams,
        swap_protocol: SwapProtocol,
    ) -> BehaviourOutEvent {
        match swap_protocol {
            SwapProtocol::HbitHerc20 => BehaviourOutEvent::HbitHerc20 {
                alpha: hbit::Params {
                    network: common.bitcoin_network,
                    asset: common.bitcoin_quantity,
                    redeem_identity: bob.bitcoin_identity,
                    refund_identity: alice.bitcoin_identity,
                    expiry: Timestamp::from(common.bitcoin_absolute_expiry),
                    secret_hash: alice.secret_hash,
                },
                beta: herc20::Params {
                    asset: asset::Erc20 {
                        token_contract: Default::default(),
                        quantity: common.ethereum_quantity,
                    },
                    redeem_identity: alice.ethereum_identity,
                    refund_identity: bob.ethereum_identity,
                    expiry: Timestamp::from(common.ethereum_absolute_expiry),
                    secret_hash: alice.secret_hash,
                    chain_id: common.ethereum_chain_id,
                },
            },
            SwapProtocol::Herc20Hbit => BehaviourOutEvent::Herc20Hbit {
                alpha: herc20::Params {
                    asset: asset::Erc20 {
                        token_contract: Default::default(),
                        quantity: common.ethereum_quantity,
                    },
                    redeem_identity: bob.ethereum_identity,
                    refund_identity: alice.ethereum_identity,
                    expiry: Timestamp::from(common.ethereum_absolute_expiry),
                    secret_hash: alice.secret_hash,
                    chain_id: common.ethereum_chain_id,
                },
                beta: hbit::Params {
                    network: common.bitcoin_network,
                    asset: common.bitcoin_quantity,
                    redeem_identity: alice.bitcoin_identity,
                    refund_identity: bob.bitcoin_identity,
                    expiry: Timestamp::from(common.bitcoin_absolute_expiry),
                    secret_hash: alice.secret_hash,
                },
            },
        }
    }
}

enum SwapProtocol {
    HbitHerc20,
    Herc20Hbit,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct SetupSwap {
    hbit_herc20: RequestResponse<Codec<HbitHerc20Protocol>>,
    herc20_hbit: RequestResponse<Codec<Herc20HbitProtocol>>,
    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
    #[behaviour(ignore)]
    swap_data: HashMap<CommonParams, RoleDependentParams>,
}

impl Default for SetupSwap {
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
        }
    }
}

impl SetupSwap {
    pub fn alice_send_hbit_herc20(
        &mut self,
        to: &PeerId,
        bitcoin_identity: identity::Bitcoin,
        ethereum_identity: identity::Ethereum,
        secret_hash: SecretHash,
        common: CommonParams,
    ) -> anyhow::Result<()> {
        self.alice_send(
            to,
            bitcoin_identity,
            ethereum_identity,
            secret_hash,
            common,
            SwapProtocol::HbitHerc20,
        )
    }

    pub fn alice_send_herc20_hbit(
        &mut self,
        to: &PeerId,
        bitcoin_identity: identity::Bitcoin,
        ethereum_identity: identity::Ethereum,
        secret_hash: SecretHash,
        common: CommonParams,
    ) -> anyhow::Result<()> {
        self.alice_send(
            to,
            bitcoin_identity,
            ethereum_identity,
            secret_hash,
            common,
            SwapProtocol::Herc20Hbit,
        )
    }

    pub fn bob_send_hbit_herc20(
        &mut self,
        to: &PeerId,
        bitcoin_identity: identity::Bitcoin,
        ethereum_identity: identity::Ethereum,
        common: CommonParams,
    ) -> anyhow::Result<()> {
        self.bob_send(
            to,
            bitcoin_identity,
            ethereum_identity,
            common,
            SwapProtocol::HbitHerc20,
        )
    }

    pub fn bob_send_herc20_hbit(
        &mut self,
        to: &PeerId,
        bitcoin_identity: identity::Bitcoin,
        ethereum_identity: identity::Ethereum,
        common: CommonParams,
    ) -> anyhow::Result<()> {
        self.bob_send(
            to,
            bitcoin_identity,
            ethereum_identity,
            common,
            SwapProtocol::Herc20Hbit,
        )
    }
    fn alice_send(
        &mut self,
        to: &PeerId,
        bitcoin_identity: identity::Bitcoin,
        ethereum_identity: identity::Ethereum,
        secret_hash: SecretHash,
        common: CommonParams,
        swap_protocol: SwapProtocol,
    ) -> anyhow::Result<()> {
        let alice = AliceParams {
            ethereum_identity,
            bitcoin_identity,
            secret_hash,
        };
        match self.swap_data.get(&common) {
            Some(RoleDependentParams::Bob(bob)) => {
                self.events
                    .push_back(BehaviourOutEvent::new(common, &alice, bob, swap_protocol));
                Ok(())
            }
            Some(RoleDependentParams::Alice(_)) => Err(anyhow::Error::from(RoleMistmatch {
                peer: to.clone(),
                role: Role::Alice,
                id: common,
            })),
            None => {
                self.swap_data
                    .insert(common.clone(), RoleDependentParams::Alice(alice));
                let _ = match swap_protocol {
                    SwapProtocol::Herc20Hbit => self.herc20_hbit.send_request(to, Message::Alice {
                        _marker: Default::default(),
                        alice,
                        common,
                    }),
                    SwapProtocol::HbitHerc20 => self.hbit_herc20.send_request(to, Message::Alice {
                        _marker: Default::default(),
                        alice,
                        common,
                    }),
                };
                Ok(())
            }
        }
    }

    fn bob_send(
        &mut self,
        to: &PeerId,
        bitcoin_identity: identity::Bitcoin,
        ethereum_identity: identity::Ethereum,
        common: CommonParams,
        swap_protocol: SwapProtocol,
    ) -> anyhow::Result<()> {
        let bob = BobParams {
            ethereum_identity,
            bitcoin_identity,
        };
        match self.swap_data.get(&common) {
            Some(RoleDependentParams::Alice(alice)) => {
                self.events
                    .push_back(BehaviourOutEvent::new(common, &alice, &bob, swap_protocol));
                Ok(())
            }
            Some(RoleDependentParams::Bob(_)) => Err(anyhow::Error::from(RoleMistmatch {
                peer: to.clone(),
                role: Role::Alice,
                id: common,
            })),
            None => {
                self.swap_data
                    .insert(common.clone(), RoleDependentParams::Bob(bob));
                let _ = match swap_protocol {
                    SwapProtocol::Herc20Hbit => self.herc20_hbit.send_request(to, Message::Bob {
                        _marker: Default::default(),
                        bob,
                        common,
                    }),
                    SwapProtocol::HbitHerc20 => self.hbit_herc20.send_request(to, Message::Bob {
                        _marker: Default::default(),
                        bob,
                        common,
                    }),
                };
                Ok(())
            }
        }
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
            Some(RoleDependentParams::Alice(alice)) => self
                .events
                .push_back(BehaviourOutEvent::new(common, alice, &bob, swap_protocol)),
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
                self.events
                    .push_back(BehaviourOutEvent::new(common, &alice, bob, swap_protocol))
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
    ) -> Poll<NetworkBehaviourAction<InEvent, BehaviourOutEvent>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<Message<HbitHerc20Protocol>, ()>>
    for SetupSwap
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

impl NetworkBehaviourEventProcess<RequestResponseEvent<Message<Herc20HbitProtocol>, ()>>
    for SetupSwap
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
    pub ethereum_quantity: asset::Erc20Quantity,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub bitcoin_quantity: asset::Bitcoin,
    pub ethereum_absolute_expiry: u32,
    pub bitcoin_absolute_expiry: u32,
    pub ethereum_chain_id: ChainId,
    pub bitcoin_network: Network,
}

impl Display for CommonParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct AliceParams {
    ethereum_identity: identity::Ethereum,
    bitcoin_identity: identity::Bitcoin,
    secret_hash: SecretHash,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BobParams {
    ethereum_identity: identity::Ethereum,
    bitcoin_identity: identity::Bitcoin,
}

#[derive(Debug, Copy, Clone)]
pub enum RoleDependentParams {
    Alice(AliceParams),
    Bob(BobParams),
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

    /// Writes a take order request to the given I/O stream.
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
        let (mut alice_swarm, _, alice_id) = new_swarm(|_| SetupSwap::default());
        let (mut bob_swarm, _, bob_id) = new_swarm(|_| SetupSwap::default());
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
            ethereum_quantity: asset::Erc20Quantity::zero(),
            bitcoin_quantity: asset::Bitcoin::from_sat(0),
            ethereum_absolute_expiry: 0,
            bitcoin_absolute_expiry: 0,
            ethereum_chain_id: ChainId::regtest(),
            bitcoin_network: Network::Regtest,
        };

        bob_swarm
            .bob_send_hbit_herc20(
                &alice_id,
                bitcoin_identity,
                ethereum_identity,
                common.clone(),
            )
            .expect("bob failed to send");
        alice_swarm
            .alice_send_hbit_herc20(
                &bob_id,
                bitcoin_identity,
                ethereum_identity,
                secret_hash,
                common,
            )
            .expect("alice failed to send");

        assert_both_confirmed(alice_swarm.next(), bob_swarm.next()).await;
    }

    async fn assert_both_confirmed(
        alice_event: impl Future<Output = BehaviourOutEvent>,
        bob_event: impl Future<Output = BehaviourOutEvent>,
    ) {
        match await_events_or_timeout(alice_event, bob_event).await {
            (
                BehaviourOutEvent::HbitHerc20 {
                    alpha: alice_alpha,
                    beta: alice_beta,
                },
                BehaviourOutEvent::HbitHerc20 {
                    alpha: bob_alpha,
                    beta: bob_beta
                },
            ) => {
                assert_eq!(alice_alpha, bob_alpha);
                assert_eq!(alice_beta, bob_beta);
            }
            (alice_event, bob_event) => panic!("expected both parties to confirm the swap but alice emitted {:?} and bob emitted {:?}", alice_event, bob_event),
        }
    }
}
