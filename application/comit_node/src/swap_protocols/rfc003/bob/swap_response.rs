use comit_client::SwapReject;
use ethereum_support;
use swap_protocols::rfc003::{ethereum::Seconds, state_machine::StateMachineResponse};

#[derive(Clone, Debug, PartialEq)]
pub enum SwapResponseKind {
    BitcoinEthereum(
        Result<
            StateMachineResponse<secp256k1_support::KeyPair, ethereum_support::Address, Seconds>,
            SwapReject,
        >,
    ),
}

impl
    From<
        Result<
            StateMachineResponse<secp256k1_support::KeyPair, ethereum_support::Address, Seconds>,
            SwapReject,
        >,
    > for SwapResponseKind
{
    fn from(
        result: Result<
            StateMachineResponse<secp256k1_support::KeyPair, ethereum_support::Address, Seconds>,
            SwapReject,
        >,
    ) -> Self {
        SwapResponseKind::BitcoinEthereum(result)
    }
}
