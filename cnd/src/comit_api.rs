use crate::{
    asset::{self, AssetKind},
    libp2p_comit_ext::{FromHeader, ToHeader},
    swap_protocols::{
        ledger,
        rfc003::{messages::Decision, SwapId},
        SwapProtocol,
    },
};
use libp2p_comit::frame::Header;
use serde::de::Error;
use std::str::FromStr;

#[derive(Clone, Copy, derivative::Derivative, PartialEq)]
#[derivative(Debug = "transparent")]
pub enum LedgerKind {
    Bitcoin(ledger::Bitcoin),
    Ethereum(ledger::Ethereum),
}

impl FromHeader for LedgerKind {
    fn from_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "bitcoin" => match header.take_parameter::<String>("network")?.as_ref() {
                "mainnet" => LedgerKind::Bitcoin(ledger::Bitcoin::Mainnet),
                "testnet" => LedgerKind::Bitcoin(ledger::Bitcoin::Testnet),
                "regtest" => LedgerKind::Bitcoin(ledger::Bitcoin::Regtest),
                _ => {
                    return Err(serde_json::Error::custom(
                        "unexpected bitcoin network variant",
                    ))
                }
            },
            "ethereum" => {
                LedgerKind::Ethereum(ledger::Ethereum::new(header.take_parameter("chain_id")?))
            }
            unknown => {
                return Err(serde_json::Error::custom(format!(
                    "unknown ledger: {}",
                    unknown
                )))
            }
        })
    }
}

impl ToHeader for LedgerKind {
    fn to_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            LedgerKind::Bitcoin(ledger::Bitcoin::Mainnet) => {
                Header::with_str_value("bitcoin").with_parameter("network", "mainnet")?
            }
            LedgerKind::Bitcoin(ledger::Bitcoin::Testnet) => {
                Header::with_str_value("bitcoin").with_parameter("network", "testnet")?
            }

            LedgerKind::Bitcoin(ledger::Bitcoin::Regtest) => {
                Header::with_str_value("bitcoin").with_parameter("network", "regtest")?
            }

            LedgerKind::Ethereum(ethereum) => {
                Header::with_str_value("ethereum").with_parameter("chain_id", ethereum.chain_id)?
            }
        })
    }
}

impl FromHeader for SwapId {
    fn from_header(header: Header) -> Result<Self, serde_json::Error> {
        header.value::<SwapId>()
    }
}

impl ToHeader for SwapId {
    fn to_header(&self) -> Result<Header, serde_json::Error> {
        Header::with_value(self)
    }
}

impl FromHeader for SwapProtocol {
    fn from_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "comit-rfc-003" => SwapProtocol::Rfc003(header.take_parameter("hash_function")?),
            unknown => {
                return Err(serde_json::Error::custom(format!(
                    "unknown swap protocol: {}",
                    unknown
                )))
            }
        })
    }
}

impl ToHeader for SwapProtocol {
    fn to_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            SwapProtocol::Rfc003(hash_function) => Header::with_str_value("comit-rfc-003")
                .with_parameter("hash_function", hash_function)?,
        })
    }
}

impl FromHeader for AssetKind {
    fn from_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "bitcoin" => {
                let quantity = header.take_parameter::<String>("quantity")?;
                let quantity =
                    u64::from_str(quantity.as_str()).map_err(serde_json::Error::custom)?;
                let amount = asset::Bitcoin::from_sat(quantity);

                AssetKind::Bitcoin(amount)
            }
            "ether" => AssetKind::Ether(header.take_parameter("quantity")?),
            "erc20" => AssetKind::Erc20(asset::Erc20::new(
                header.take_parameter("address")?,
                header.take_parameter("quantity")?,
            )),
            unknown => {
                return Err(serde_json::Error::custom(format!(
                    "unknown asset: {}",
                    unknown
                )))
            }
        })
    }
}

impl ToHeader for AssetKind {
    fn to_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            AssetKind::Bitcoin(bitcoin) => Header::with_str_value("bitcoin")
                .with_parameter("quantity", bitcoin.as_sat().to_string())?,
            AssetKind::Ether(ether) => {
                Header::with_str_value("ether").with_parameter("quantity", ether)?
            }
            AssetKind::Erc20(erc20) => Header::with_str_value("erc20")
                .with_parameter("address", erc20.token_contract.clone())?
                .with_parameter("quantity", erc20.quantity.clone())?,
        })
    }
}

impl ToHeader for Decision {
    fn to_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            Decision::Accepted => Header::with_str_value("accepted"),
            Decision::Declined => Header::with_str_value("declined"),
        })
    }
}

impl FromHeader for Decision {
    fn from_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "accepted" => Decision::Accepted,
            "declined" => Decision::Declined,
            _ => return Err(::serde::de::Error::custom("failed to deserialize decision")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset::ethereum::FromWei,
        ethereum::{Address, ChainId, U256},
        swap_protocols::HashFunction,
    };

    #[test]
    fn erc20_quantity_to_header() -> Result<(), serde_json::Error> {
        let quantity = asset::Erc20::new(
            Address::from([0u8; 20]),
            asset::Erc20Quantity::from_wei(U256::from(100_000_000_000_000u64)),
        );
        let header = AssetKind::from(quantity).to_header()?;

        assert_eq!(
            header,
            Header::with_str_value("erc20")
                .with_parameter("quantity", "100000000000000")?
                .with_parameter("address", "0x0000000000000000000000000000000000000000")?
        );

        Ok(())
    }

    #[test]
    fn swap_protocol_to_header() {
        // From comit-network/RFCs/RFC-003-SWAP-Basic.md SWAP REQUEST example.
        //
        // "protocol": {
        //     "value": "comit-rfc-003",
        //     "parameters": {
        //       "hash_function": "SHA-256"
        //     }
        // }
        let header = Header::with_str_value("comit-rfc-003")
            .with_parameter("hash_function", "SHA-256")
            .unwrap();

        let protocol = SwapProtocol::Rfc003(HashFunction::Sha256);
        let protocol = protocol.to_header().unwrap();

        assert_eq!(header, protocol);
    }

    #[test]
    fn bitcoin_quantity_to_header() {
        let quantity = asset::Bitcoin::from_sat(100_000_000);
        let header = AssetKind::from(quantity).to_header().unwrap();

        assert_eq!(
            header,
            Header::with_str_value("bitcoin")
                .with_parameter("quantity", "100000000")
                .unwrap()
        );
    }

    #[test]
    fn bitcoin_quantity_from_header() {
        let header = Header::with_str_value("bitcoin")
            .with_parameter("quantity", "100000000")
            .unwrap();

        let quantity = AssetKind::from_header(header).unwrap();
        let amount = asset::Bitcoin::from_sat(100_000_000);
        assert_eq!(quantity, AssetKind::Bitcoin(amount));
    }

    #[test]
    fn ethereum_ledger_to_header() {
        let ledger = LedgerKind::Ethereum(ledger::Ethereum::new(ChainId::ropsten()));
        let header = ledger.to_header().unwrap();

        assert_eq!(
            header,
            Header::with_str_value("ethereum")
                .with_parameter("chain_id", 3)
                .unwrap()
        );
    }

    #[test]
    fn bitcoin_ledger_to_header_roundtrip() {
        let ledgerkinds = vec![
            LedgerKind::Bitcoin(ledger::Bitcoin::Mainnet),
            LedgerKind::Bitcoin(ledger::Bitcoin::Testnet),
            LedgerKind::Bitcoin(ledger::Bitcoin::Regtest),
        ];

        let headers = vec![
            Header::with_str_value("bitcoin")
                .with_parameter("network", "mainnet")
                .unwrap(),
            Header::with_str_value("bitcoin")
                .with_parameter("network", "testnet")
                .unwrap(),
            Header::with_str_value("bitcoin")
                .with_parameter("network", "regtest")
                .unwrap(),
        ];

        let serialized_headers = ledgerkinds
            .iter()
            .map(|ledger| ledger.to_header())
            .collect::<Result<Vec<Header>, serde_json::Error>>()
            .unwrap();

        let constructed_ledgerkinds = serialized_headers
            .iter()
            .map(|header| LedgerKind::from_header(header.clone()))
            .collect::<Result<Vec<LedgerKind>, serde_json::Error>>()
            .unwrap();

        assert_eq!(serialized_headers, headers);
        assert_eq!(constructed_ledgerkinds, ledgerkinds);
    }
}
