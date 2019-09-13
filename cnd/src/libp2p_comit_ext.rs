use libp2p_comit::frame::Header;

pub trait FromHeader
where
    Self: Sized,
{
    fn from_header(header: Header) -> Result<Self, serde_json::Error>;
}

pub trait ToHeader {
    fn to_header(&self) -> Result<Header, serde_json::Error>;
}
