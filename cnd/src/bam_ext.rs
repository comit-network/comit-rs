use libp2p_comit::frame::Header;

pub trait FromBamHeader
where
    Self: Sized,
{
    fn from_bam_header(header: Header) -> Result<Self, serde_json::Error>;
}

pub trait ToBamHeader {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error>;
}
