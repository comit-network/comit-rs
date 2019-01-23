use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::SwapDeclineReason,
    swap_protocols::SwapProtocols,
};
use bam::json::Header;

pub mod rfc003;

mod ledger_impls {
    use crate::{
        bam_ext::{FromBamHeader, ToBamHeader},
        swap_protocols::ledger::{Bitcoin, Ethereum, Ledgers},
    };
    use bam::json::Header;

    impl FromBamHeader for Ledgers {
        fn from_bam_header(mut header: Header) -> Result<Self, serde_json::Error> {
            Ok(match header.value::<String>()?.as_str() {
                "Bitcoin" => Ledgers::Bitcoin(Bitcoin::new(header.take_parameter("network")?)),
                "Ethereum" => Ledgers::Ethereum(Ethereum::new(header.take_parameter("network")?)),
                other => Ledgers::Unknown {
                    name: other.to_string(),
                },
            })
        }
    }

    impl ToBamHeader for Ledgers {
        fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
            Ok(match self {
                Ledgers::Bitcoin(bitcoin) => {
                    Header::with_str_value("Bitcoin").with_parameter("network", bitcoin.network)?
                }
                Ledgers::Ethereum(ethereum) => Header::with_str_value("Ethereum")
                    .with_parameter("network", ethereum.network)?,
                Ledgers::Unknown { name } => panic!(
                    "make {} a supported ledger before you call to_bam_header on it",
                    name
                ),
            })
        }
    }
}

mod asset_impls {
    use crate::{
        bam_ext::{FromBamHeader, ToBamHeader},
        swap_protocols::asset::Assets,
    };
    use bam::json::Header;
    use ethereum_support::Erc20Token;

    impl FromBamHeader for Assets {
        fn from_bam_header(mut header: Header) -> Result<Self, serde_json::Error> {
            Ok(match header.value::<String>()?.as_str() {
                "Bitcoin" => Assets::Bitcoin(header.take_parameter("quantity")?),
                "Ether" => Assets::Ether(header.take_parameter("quantity")?),
                "ERC20" => Assets::Erc20(Erc20Token::new(
                    header.take_parameter("address")?,
                    header.take_parameter("quantity")?,
                )),
                other => Assets::Unknown {
                    name: other.to_string(),
                },
            })
        }
    }

    impl ToBamHeader for Assets {
        fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
            Ok(match self {
                Assets::Bitcoin(bitcoin) => {
                    Header::with_str_value("Bitcoin").with_parameter("quantity", bitcoin)?
                }
                Assets::Ether(ether) => {
                    Header::with_str_value("Ether").with_parameter("quantity", ether)?
                }
                Assets::Erc20(erc20) => Header::with_str_value("ERC20")
                    .with_parameter("address", erc20.token_contract())?
                    .with_parameter("quantity", erc20.quantity())?,
                Assets::Unknown { name } => panic!(
                    "make {} a supported asset before you call to_bam_header on it",
                    name
                ),
            })
        }
    }
}

impl FromBamHeader for SwapProtocols {
    fn from_bam_header(header: Header) -> Result<Self, serde_json::Error> {
        Ok(match header.value::<String>()?.as_str() {
            "COMIT-RFC-003" => SwapProtocols::Rfc003,
            other => SwapProtocols::Unknown {
                name: other.to_string(),
            },
        })
    }
}

impl ToBamHeader for SwapProtocols {
    fn to_bam_header(&self) -> Result<Header, serde_json::Error> {
        match self {
            SwapProtocols::Rfc003 => Ok(Header::with_str_value("COMIT-RFC-003")),
            SwapProtocols::Unknown { name } => panic!(
                "make {} a supported protocol before you call to_bam_header on it",
                name
            ),
        }
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

    use crate::{bam_ext::ToBamHeader, swap_protocols::asset::Assets};
    use bam::json::Header;

    #[test]
    fn erc20_quantity_to_bam_header() -> Result<(), serde_json::Error> {
        let quantity = Erc20Token::new(
            Address::zero(),
            Erc20Quantity(U256::from(100_000_000_000_000u64)),
        );
        let header = Assets::from(quantity).to_bam_header()?;

        assert_eq!(
            header,
            Header::with_str_value("ERC20")
                .with_parameter("quantity", "100000000000000")?
                .with_parameter("address", "0x0000000000000000000000000000000000000000")?
        );

        Ok(())
    }

}
