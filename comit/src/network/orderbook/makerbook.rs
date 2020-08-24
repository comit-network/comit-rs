use byteorder::{BigEndian, ByteOrder};
use conquer_once::Lazy;
use libp2p::{
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, GossipsubRpc,
        MessageAuthenticity, MessageId, Topic,
    },
    identity::Keypair,
    swarm::{
        DialPeerCondition, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour, PeerId,
};
use std::{
    collections::{hash_map::DefaultHasher, VecDeque},
    hash::{Hash, Hasher},
    task::{Context, Poll},
    time::Duration,
};

static COMIT_MAKERS: Lazy<Topic> = Lazy::new(|| Topic::new("/comit/makers".to_string()));

#[derive(Debug)]
pub enum BehaviourOutEvent {
    /// The given peer is no longer available for trading the given trading
    /// pair.
    ///
    /// Connections to this peer can be closed as a result of this event.
    Logout { peer: PeerId },
}

/// A [NetworkBehaviour] for discovering peers that are likely to trade with us.
///
/// The functional scope of this [NetworkBehaviour] is to establish connections
/// to interesting peers. Once the connections are established, other modules
/// can utilize these connection for executing trading protocols.
#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "poll", out_event = "BehaviourOutEvent")]
#[allow(missing_debug_implementations)]
pub struct Makerbook {
    gossipsub: Gossipsub,

    #[behaviour(ignore)]
    actions: VecDeque<NetworkBehaviourAction<GossipsubRpc, BehaviourOutEvent>>,
}

impl Makerbook {
    pub fn new(key: Keypair) -> Self {
        let mut gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(key),
            GossipsubConfigBuilder::new()
                .heartbeat_interval(Duration::from_secs(1))
                .message_id_fn(content_based_id)
                .build(),
        );

        gossipsub.subscribe(COMIT_MAKERS.clone());

        Self {
            gossipsub,
            actions: VecDeque::default(),
        }
    }

    pub fn login(&mut self) {
        let message = serde_json::to_vec(&wire::Message::Login {
            trading_pair: wire::TradingPair::BtcDai,
        })
        .expect("serialization doesn't panic");
        if self.gossipsub.publish(&COMIT_MAKERS, message).is_err() {
            tracing::warn!("login publish message failed");
        }
    }

    pub fn logout(&mut self) {
        let message = serde_json::to_vec(&wire::Message::Logout {
            trading_pair: wire::TradingPair::BtcDai,
        })
        .expect("serialization doesn't panic");
        if self.gossipsub.publish(&COMIT_MAKERS, message).is_err() {
            tracing::warn!("logout publish message failed");
        }
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<GossipsubRpc, BehaviourOutEvent>> {
        if let Some(action) = self.actions.pop_front() {
            return Poll::Ready(action);
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for Makerbook {
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message(relayed_from, _, message) = event {
            let source = message.source.unwrap();
            let message = match serde_json::from_slice::<wire::Message>(&message.data) {
                Ok(message) => message,
                Err(e) => {
                    tracing::debug!("receives malformed message from {}: {:?}", relayed_from, e);
                    return;
                }
            };

            match message {
                wire::Message::Login { trading_pair } => {
                    tracing::info!(
                        "{} announced that they are trading {}, dialling ...",
                        source,
                        trading_pair
                    );
                    self.actions.push_back(NetworkBehaviourAction::DialPeer {
                        peer_id: source,
                        condition: DialPeerCondition::NotDialing, /* we only want to establish a
                                                                   * connection in case we don't
                                                                   * already have one */
                    });
                }
                wire::Message::Logout { trading_pair } => {
                    tracing::info!(
                        "{} is no longer available for trading {}",
                        source,
                        trading_pair
                    );
                    self.actions
                        .push_back(NetworkBehaviourAction::GenerateEvent(
                            BehaviourOutEvent::Logout { peer: source },
                        ))
                }
            }
        }
    }
}

fn content_based_id(message: &GossipsubMessage) -> MessageId {
    let mut s = DefaultHasher::new();
    message.data.hash(&mut s);
    let hash = s.finish();

    let mut buf = [0; 8];
    // FIXME: big or little, does it matter?
    BigEndian::write_u64(&mut buf, hash);

    MessageId::new(&buf)
}

mod wire {
    use serde::{Deserialize, Serialize};
    use std::fmt;

    /// All messages sent to the `/comit/makers` topic.
    #[derive(Debug, Serialize, Deserialize)]
    pub enum Message {
        /// Informs all subscribers that the source peer is online and ready to
        /// trade the given trading pair.
        Login { trading_pair: TradingPair },
        /// Informs all subscribers that the source peers is going offline and
        /// no longer available for trading the given trading pair.
        Logout { trading_pair: TradingPair },
    }

    /// Defines the set of trading pairs that we support.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum TradingPair {
        #[serde(rename = "BTC/DAI")]
        BtcDai,
    }

    impl fmt::Display for TradingPair {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", match self {
                TradingPair::BtcDai => "BTC/DAI",
            })
        }
    }
}
