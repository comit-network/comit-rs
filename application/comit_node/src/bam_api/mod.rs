use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::SwapDeclineReason,
    swap_protocols::{
        asset::AssetKind,
        ledger::{Bitcoin, Ethereum, LedgerKind},
        SwapProtocol,
    },
};
use bam::json::Header;
use ethereum_support::Erc20Token;
use std::fmt;

fn fail_serialize_unknown<D: fmt::Debug>(unknown: D) -> serde_json::Error {
    serde::de::Error::custom(format!("serialization of {:?} is undefined.", unknown))
}

impl FromBamHeader for LedgerKind {
    fn from_bam_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "bitcoin" => LedgerKind::Bitcoin(Bitcoin::new(header.take_parameter("network")?)),
            "ethereum" => LedgerKind::Ethereum(Ethereum::new(header.take_parameter("network")?)),
            other => LedgerKind::Unknown(other.to_string()),
        })
    }
}

impl ToBamHeader for LedgerKind {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
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

impl FromBamHeader for SwapProtocol {
    fn from_bam_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "comit-rfc-003" => SwapProtocol::Rfc003,
            other => SwapProtocol::Unknown(other.to_string()),
        })
    }
}

impl ToBamHeader for SwapProtocol {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            SwapProtocol::Rfc003 => Header::with_str_value("comit-rfc-003"),
            unknown @ SwapProtocol::Unknown(_) => return Err(fail_serialize_unknown(unknown)),
        })
    }
}

impl FromBamHeader for AssetKind {
    fn from_bam_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "bitcoin" => AssetKind::Bitcoin(header.take_parameter("quantity")?),
            "ether" => AssetKind::Ether(header.take_parameter("quantity")?),
            "erc20" => AssetKind::Erc20(Erc20Token::new(
                header.take_parameter("address")?,
                header.take_parameter("quantity")?,
            )),
            other => AssetKind::Unknown(other.to_string()),
        })
    }
}

impl ToBamHeader for AssetKind {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            AssetKind::Bitcoin(bitcoin) => {
                Header::with_str_value("bitcoin").with_parameter("quantity", bitcoin)?
            }
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

impl FromBamHeader for SwapDeclineReason {
    fn from_bam_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "bad-rate" => SwapDeclineReason::BadRate,
            other => SwapDeclineReason::Unknown(other.to_string()),
        })
    }
}

impl ToBamHeader for SwapDeclineReason {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            SwapDeclineReason::BadRate => Header::with_str_value("bad-rate"),
            unknown @ SwapDeclineReason::Unknown(_) => return Err(fail_serialize_unknown(unknown)),
        })
    }
}

#[cfg(test)]
mod tests {

    use ethereum_support::{Address, Erc20Quantity, Erc20Token, U256};

    use crate::{
        bam_ext::ToBamHeader,
        swap_protocols::{asset::AssetKind, LedgerKind, SwapProtocol},
    };
    use bam::json::Header;
    use spectral::prelude::*;

    #[test]
    fn erc20_quantity_to_bam_header() -> Result<(), serde_json::Error> {
        let quantity = Erc20Token::new(
            Address::zero(),
            Erc20Quantity(U256::from(100_000_000_000_000u64)),
        );
        let header = AssetKind::from(quantity).to_bam_header()?;

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

        let header = ledger_kind.to_bam_header();

        assert_that(&header).is_err();
    }

    #[test]
    fn swap_protocol_to_bam_header() {
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

        let protocol = SwapProtocol::Rfc003;
        let protocol = protocol.to_bam_header().unwrap();

        assert_eq!(header, protocol);
    }
}
