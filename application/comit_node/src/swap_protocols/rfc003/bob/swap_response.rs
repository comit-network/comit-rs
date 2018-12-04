use bitcoin_support;
use comit_client::SwapReject;
use ethereum_support;
use std_ext::time::Seconds;
use swap_protocols::rfc003::state_machine::StateMachineResponse;

#[derive(Clone, Debug, PartialEq)]
pub enum SwapResponseKind {
    BitcoinEthereum(
        Result<
            StateMachineResponse<secp256k1_support::KeyPair, ethereum_support::Address, Seconds>,
            SwapReject,
        >,
    ),

    EthereumLightning(
        Result<
            StateMachineResponse<
                ethereum_support::Address,
                secp256k1_support::PublicKey,
                bitcoin_support::Blocks,
            >,
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

impl
    From<
        Result<
            StateMachineResponse<
                ethereum_support::Address,
                secp256k1_support::PublicKey,
                bitcoin_support::Blocks,
            >,
            SwapReject,
        >,
    > for SwapResponseKind
{
    fn from(
        result: Result<
            StateMachineResponse<
                ethereum_support::Address,
                secp256k1_support::PublicKey,
                bitcoin_support::Blocks,
            >,
            SwapReject,
        >,
    ) -> Self {
        SwapResponseKind::EthereumLightning(result)
    }
}
