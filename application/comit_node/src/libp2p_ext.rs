use libp2p::{Multiaddr, PeerId};

pub trait BamPeers: Send + Sync + 'static {
    fn bam_peers(&self) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static>;
}
