use bam_api::header::{Error, FromBamHeader, Header, ToBamHeader};
use swap_protocols::SwapProtocols;

pub mod rfc003;

pub mod header;

mod ledger_impls {
    use bam_api::header::{Error, FromBamHeader, Header, ToBamHeader};
    use swap_protocols::ledger::{Bitcoin, Ethereum};

    impl FromBamHeader for Bitcoin {
        fn from_bam_header(mut header: Header) -> Result<Self, Error> {
            let _ = header.has_value("Bitcoin")?;

            Ok(Bitcoin {
                network: header.parameter("network")?,
            })
        }
    }

    impl ToBamHeader for Bitcoin {
        fn to_bam_header(&self) -> Result<Header, Error> {
            Ok(Header::with_value("Bitcoin").with_parameter("network", self.network)?)
        }
    }

    impl FromBamHeader for Ethereum {
        fn from_bam_header(header: Header) -> Result<Self, Error> {
            let _ = header.has_value("Ethereum")?;

            Ok(Ethereum {})
        }
    }

    impl ToBamHeader for Ethereum {
        fn to_bam_header(&self) -> Result<Header, Error> {
            Ok(Header::with_value("Ethereum"))
        }
    }
}

mod asset_impls {
    use bam_api::header::{Error, FromBamHeader, Header, ToBamHeader};
    use bitcoin_support::BitcoinQuantity;
    use ethereum_support::{
        web3::types::U256, Erc20Quantity, EtherQuantity, FromDecimalStr, ToBigInt,
    };

    impl FromBamHeader for BitcoinQuantity {
        fn from_bam_header(mut header: Header) -> Result<Self, Error> {
            let _ = header.has_value("Bitcoin")?;

            Ok(header.parameter("quantity")?)
        }
    }

    impl ToBamHeader for BitcoinQuantity {
        fn to_bam_header(&self) -> Result<Header, Error> {
            Ok(Header::with_value("Bitcoin").with_parameter("quantity", self)?)
        }
    }

    impl FromBamHeader for EtherQuantity {
        fn from_bam_header(mut header: Header) -> Result<Self, Error> {
            let _ = header.has_value("Ether")?;

            Ok(header.parameter("quantity")?)
        }
    }

    impl ToBamHeader for EtherQuantity {
        fn to_bam_header(&self) -> Result<Header, Error> {
            Ok(Header::with_value("Ether").with_parameter("quantity", self)?)
        }
    }

    impl FromBamHeader for Erc20Quantity {
        fn from_bam_header(mut header: Header) -> Result<Self, Error> {
            let _ = header.has_value("ERC20")?;

            let amount: String = header.parameter("quantity")?;

            Ok(Erc20Quantity::new(
                header.parameter("address")?,
                U256::from_decimal_str(&amount).map_err(|_| Error::Parsing)?,
            ))
        }
    }

    impl ToBamHeader for Erc20Quantity {
        fn to_bam_header(&self) -> Result<Header, Error> {
            Ok(Header::with_value("ERC20")
                .with_parameter("address", self.token_contract())?
                .with_parameter("quantity", format!("{}", self.quantity().to_bigint()))?)
        }
    }
}

impl FromBamHeader for SwapProtocols {
    fn from_bam_header(header: Header) -> Result<Self, Error> {
        match header.value() {
            "COMIT-RFC-003" => Ok(SwapProtocols::Rfc003),
            _ => Err(Error::WrongValue),
        }
    }
}

impl ToBamHeader for SwapProtocols {
    fn to_bam_header(&self) -> Result<Header, Error> {
        match self {
            SwapProtocols::Rfc003 => Ok(Header::with_value("COMIT-RFC-003")),
        }
    }
}

#[cfg(test)]
mod tests {

    use bam_api::header::{Error, Header, ToBamHeader};
    use ethereum_support::{Address, Erc20Quantity, U256};

    #[test]
    fn erc20_quantity_to_bam_header() -> Result<(), Error> {
        let quantity = Erc20Quantity::new(Address::zero(), U256::from(100_000_000_000_000u64));
        let header = quantity.to_bam_header()?;

        assert_eq!(
            header,
            Header::with_value("ERC20")
                .with_parameter("quantity", "100000000000000")?
                .with_parameter("address", "0x0000000000000000000000000000000000000000")?
        );

        Ok(())
    }

}
