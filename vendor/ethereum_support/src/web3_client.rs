use std::ops::Deref;
use web3_crate::{
    api::Web3,
    transports::{EventLoopHandle, Http},
};

pub struct Web3Client {
    _event_loop: EventLoopHandle,
    web3: Web3<Http>,
}

impl Web3Client {
    pub fn new(endpoint: String) -> Self {
        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let web3 = Web3::new(transport);

        Web3Client { _event_loop, web3 }
    }
}

impl Deref for Web3Client {
    type Target = Web3<Http>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.web3
    }
}
