#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, Hash, IntoStaticStr)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[strum(serialize = "mainnet")]
    Mainnet,
    #[strum(serialize = "regtest")]
    Regtest,
    #[strum(serialize = "ropsten")]
    Ropsten,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn string_serialize() {
        let mainnet: &'static str = Network::Mainnet.into();
        let regtest: &'static str = Network::Regtest.into();
        let ropsten: &'static str = Network::Ropsten.into();

        assert_eq!(mainnet, "mainnet");
        assert_eq!(regtest, "regtest");
        assert_eq!(ropsten, "ropsten");
    }
}
