use crate::{
    libp2p_comit_ext::{FromHeader, ToHeader},
    swap_protocols::{
        asset::AssetKind,
        ledger::{Bitcoin, Ethereum, LedgerKind},
        rfc003::messages::Decision,
        SwapId, SwapProtocol,
    },
};
use bitcoin_support::amount::Denomination;
use ethereum_support::Erc20Token;
use libp2p_comit::frame::Header;
use serde::de::Error;
use std::fmt;

fn fail_serialize_unknown<D: fmt::Debug>(unknown: D) -> serde_json::Error {
    ::serde::de::Error::custom(format!("serialization of {:?} is undefined.", unknown))
}

impl FromHeader for LedgerKind {
    fn from_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "bitcoin" => LedgerKind::Bitcoin(Bitcoin::new(header.take_parameter("network")?)),
            "ethereum" => LedgerKind::Ethereum(Ethereum::new(header.take_parameter("network")?)),
            other => LedgerKind::Unknown(other.to_string()),
        })
    }
}

impl ToHeader for LedgerKind {
    fn to_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            LedgerKind::Bitcoin(bitcoin) => {
                Header::with_str_value("bitcoin").with_parameter("network", bitcoin.network)?
            }
            LedgerKind::Ethereum(ethereum) => {
                Header::with_str_value("ethereum").with_parameter("network", ethereum.network)?
            }
            unknown @ LedgerKind::Unknown(_) => return Err(fail_serialize_unknown(unknown)),
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
            other => SwapProtocol::Unknown(other.to_string()),
        })
    }
}

impl ToHeader for SwapProtocol {
    fn to_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            SwapProtocol::Rfc003(hash_function) => Header::with_str_value("comit-rfc-003")
                .with_parameter("hash_function", hash_function)?,
            unknown @ SwapProtocol::Unknown(_) => return Err(fail_serialize_unknown(unknown)),
        })
    }
}

impl FromHeader for AssetKind {
    fn from_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "bitcoin" => {
                let quantity = header.take_parameter::<String>("quantity")?;
                let amount =
                    bitcoin_support::Amount::from_str_in(quantity.as_str(), Denomination::Satoshi)
                        .map_err(|e| serde_json::Error::custom(e.to_string()))?;

                AssetKind::Bitcoin(amount)
            }
            "ether" => AssetKind::Ether(header.take_parameter("quantity")?),
            "erc20" => AssetKind::Erc20(Erc20Token::new(
                header.take_parameter("address")?,
                header.take_parameter("quantity")?,
            )),
            other => AssetKind::Unknown(other.to_string()),
        })
    }
}

impl ToHeader for AssetKind {
    fn to_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            AssetKind::Bitcoin(bitcoin) => Header::with_str_value("bitcoin")
                .with_parameter("quantity", format!("{}", bitcoin.as_sat()))?,
            AssetKind::Ether(ether) => {
                Header::with_str_value("ether").with_parameter("quantity", ether)?
            }
            AssetKind::Erc20(erc20) => Header::with_str_value("erc20")
                .with_parameter("address", erc20.token_contract)?
                .with_parameter("quantity", erc20.quantity)?,
            unknown @ AssetKind::Unknown(_) => return Err(fail_serialize_unknown(unknown)),
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
    use crate::{
        libp2p_comit_ext::{FromHeader, ToHeader},
        swap_protocols::{asset::AssetKind, HashFunction, LedgerKind, SwapProtocol},
    };
    use bitcoin_support::Amount;
    use ethereum_support::{Address, Erc20Quantity, Erc20Token, U256};
    use libp2p_comit::frame::Header;
    use spectral::prelude::*;

    #[test]
    fn erc20_quantity_to_header() -> Result<(), serde_json::Error> {
        let quantity = Erc20Token::new(
            Address::zero(),
            Erc20Quantity(U256::from(100_000_000_000_000u64)),
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
    fn serializing_unknown_ledgerkind_doesnt_panic() {
        let ledger_kind = LedgerKind::Unknown("USD".to_string());

        let header = ledger_kind.to_header();

        assert_that(&header).is_err();
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
        let quantity = Amount::from_btc(1.0).unwrap();
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
        let amount = Amount::from_btc(1.0).unwrap();
        assert_eq!(quantity, AssetKind::Bitcoin(amount));
    }
}
