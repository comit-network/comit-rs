use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::SwapDeclineReason,
    swap_protocols::{
        asset::AssetKind,
        ledger::{Bitcoin, Ethereum, LedgerKind},
        SwapProtocols,
    },
};
use bam::json::Header;
use ethereum_support::Erc20Token;

pub mod rfc003;

impl FromBamHeader for LedgerKind {
    fn from_bam_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "Bitcoin" => LedgerKind::Bitcoin(Bitcoin::new(header.take_parameter("network")?)),
            "Ethereum" => LedgerKind::Ethereum(Ethereum::new(header.take_parameter("network")?)),
            other => LedgerKind::Unknown(other.to_string()),
        })
    }
}

impl ToBamHeader for LedgerKind {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
        Ok(match self {
            LedgerKind::Bitcoin(bitcoin) => {
                Header::with_str_value("Bitcoin").with_parameter("network", bitcoin.network)?
            }
            LedgerKind::Ethereum(ethereum) => {
                Header::with_str_value("Ethereum").with_parameter("network", ethereum.network)?
            }
            LedgerKind::Unknown(name) => panic!(
                "make {} a supported ledger before you call to_bam_header on it",
                name
            ),
        })
    }
}

impl FromBamHeader for SwapProtocols {
    fn from_bam_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "COMIT-RFC-003" => SwapProtocols::Rfc003,
            other => SwapProtocols::Unknown(other.to_string()),
        })
    }
}

impl ToBamHeader for SwapProtocols {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
        match self {
            SwapProtocols::Rfc003 => Ok(Header::with_str_value("COMIT-RFC-003")),
            SwapProtocols::Unknown(name) => panic!(
                "make {} a supported protocol before you call to_bam_header on it",
                name
            ),
        }
    }
}

impl FromBamHeader for AssetKind {
    fn from_bam_header(mut header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "Bitcoin" => AssetKind::Bitcoin(header.take_parameter("quantity")?),
            "Ether" => AssetKind::Ether(header.take_parameter("quantity")?),
            "ERC20" => AssetKind::Erc20(Erc20Token::new(
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
                Header::with_str_value("Bitcoin").with_parameter("quantity", bitcoin)?
            }
            AssetKind::Ether(ether) => {
                Header::with_str_value("Ether").with_parameter("quantity", ether)?
            }
            AssetKind::Erc20(erc20) => Header::with_str_value("ERC20")
                .with_parameter("address", erc20.token_contract())?
                .with_parameter("quantity", erc20.quantity())?,
            AssetKind::Unknown(name) => panic!(
                "make {} a supported asset before you call to_bam_header on it",
                name
            ),
        })
    }
}

impl FromBamHeader for SwapDeclineReason {
    fn from_bam_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "bad-rate" => SwapDeclineReason::BadRate,
            other => SwapDeclineReason::Unknown {
                name: other.to_string(),
            },
        })
    }
}

impl ToBamHeader for SwapDeclineReason {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
        match self {
            SwapDeclineReason::BadRate => Ok(Header::with_str_value("bad-rate")),
            SwapDeclineReason::Unknown { name } => panic!(
                "make {} a supported decline reason before you call to_bam_header on it",
                name
            ),
        }
    }
}

#[cfg(test)]
mod tests {

    use ethereum_support::{Address, Erc20Quantity, Erc20Token, U256};

    use crate::{bam_ext::ToBamHeader, swap_protocols::asset::AssetKind};
    use bam::json::Header;

    #[test]
    fn erc20_quantity_to_bam_header() -> Result<(), serde_json::Error> {
        let quantity = Erc20Token::new(
            Address::zero(),
            Erc20Quantity(U256::from(100_000_000_000_000u64)),
        );
        let header = AssetKind::from(quantity).to_bam_header()?;

        assert_eq!(
            header,
            Header::with_str_value("ERC20")
                .with_parameter("quantity", "100000000000000")?
                .with_parameter("address", "0x0000000000000000000000000000000000000000")?
        );

        Ok(())
    }

}
