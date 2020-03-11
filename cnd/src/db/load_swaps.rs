use crate::{
    asset,
    db::{
        schema,
        wrapper_types::{
            custom_sql_types::{Text, U32},
            BitcoinNetwork, Erc20Amount, Ether, EthereumAddress, Satoshis,
        },
        Sqlite,
    },
    identity,
    swap_protocols::{
        ledger::{bitcoin, Ethereum},
        rfc003::{
            messages::{Accept, Request},
            SecretHash,
        },
        HashFunction, SwapId,
    },
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::{self, prelude::*, RunQueryDsl};
use impl_template::impl_template;
use schema::{
    rfc003_bitcoin_ethereum_accept_messages,
    rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages,
    rfc003_bitcoin_ethereum_bitcoin_ether_request_messages,
    rfc003_ethereum_bitcoin_accept_messages,
    rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages,
    rfc003_ethereum_bitcoin_ether_bitcoin_request_messages,
};

pub type AcceptedSwap<AL, BL, AA, BA, AI, BI> = (
    Request<AL, BL, AA, BA, AI, BI>,
    Accept<AI, BI>,
    NaiveDateTime,
);

#[async_trait]
pub trait LoadAcceptedSwap<AL, BL, AA, BA, AI, BI> {
    async fn load_accepted_swap(
        &self,
        swap_id: &SwapId,
    ) -> anyhow::Result<AcceptedSwap<AL, BL, AA, BA, AI, BI>>;
}

diesel::allow_tables_to_appear_in_same_query!(
    rfc003_bitcoin_ethereum_bitcoin_ether_request_messages,
    rfc003_bitcoin_ethereum_accept_messages
);
diesel::allow_tables_to_appear_in_same_query!(
    rfc003_ethereum_bitcoin_ether_bitcoin_request_messages,
    rfc003_ethereum_bitcoin_accept_messages
);
diesel::allow_tables_to_appear_in_same_query!(
    rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages,
    rfc003_bitcoin_ethereum_accept_messages
);
diesel::allow_tables_to_appear_in_same_query!(
    rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages,
    rfc003_ethereum_bitcoin_accept_messages
);

// Once #1862 is fully done (ie, no more networks here) we should be able to
// include this declaration in the macro and merge it with the $select fields.
#[derive(Queryable, Debug, Clone, PartialEq)]
struct BitcoinEthereumBitcoinEtherAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    bitcoin_network: Text<BitcoinNetwork>,
    ethereum_chain_id: U32,
    bitcoin_amount: Text<Satoshis>,
    ether_amount: Text<Ether>,
    hash_function: Text<HashFunction>,
    bitcoin_refund_identity: Text<::bitcoin::PublicKey>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    bitcoin_redeem_identity: Text<::bitcoin::PublicKey>,
    ethereum_refund_identity: Text<EthereumAddress>,

    at: NaiveDateTime,
}

#[impl_template]
impl From<BitcoinEthereumBitcoinEtherAcceptedSwap>
    for AcceptedSwap<
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        Ethereum,
        asset::Bitcoin,
        asset::Ether,
        identity::Bitcoin,
        identity::Ethereum,
    >
{
    fn from(record: BitcoinEthereumBitcoinEtherAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: __TYPE0__,
                beta_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                alpha_asset: record.bitcoin_amount.0.into(),
                beta_asset: record.ether_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.bitcoin_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                alpha_expiry: record.bitcoin_expiry.into(),
                beta_expiry: record.ethereum_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.bitcoin_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[impl_template]
#[async_trait]
impl
    LoadAcceptedSwap<
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        Ethereum,
        asset::Bitcoin,
        asset::Ether,
        identity::Bitcoin,
        identity::Ethereum,
    > for Sqlite
{
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<
            __TYPE0__,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > {
        use schema::{
            rfc003_bitcoin_ethereum_accept_messages as accept_messages,
            rfc003_bitcoin_ethereum_bitcoin_ether_request_messages as request_messages,
        };

        let record: BitcoinEthereumBitcoinEtherAcceptedSwap = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                request_messages::table
                    .inner_join(
                        accept_messages::table
                            .on(request_messages::swap_id.eq(accept_messages::swap_id)),
                    )
                    .select((
                        request_messages::swap_id,
                        request_messages::bitcoin_network,
                        request_messages::ethereum_chain_id,
                        request_messages::bitcoin_amount,
                        request_messages::ether_amount,
                        request_messages::hash_function,
                        request_messages::bitcoin_refund_identity,
                        request_messages::ethereum_redeem_identity,
                        request_messages::bitcoin_expiry,
                        request_messages::ethereum_expiry,
                        request_messages::secret_hash,
                        accept_messages::bitcoin_redeem_identity,
                        accept_messages::ethereum_refund_identity,
                        accept_messages::at,
                    ))
                    .filter(accept_messages::swap_id.eq(key))
                    .first(connection)
            })
            .await?;

        Ok(record.into())
    }
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct EthereumBitcoinEtherBitcoinAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: Text<BitcoinNetwork>,
    ether_amount: Text<Ether>,
    bitcoin_amount: Text<Satoshis>,
    hash_function: Text<HashFunction>,
    ethereum_refund_identity: Text<EthereumAddress>,
    bitcoin_redeem_identity: Text<::bitcoin::PublicKey>,
    ethereum_expiry: U32,
    bitcoin_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_refund_identity: Text<::bitcoin::PublicKey>,

    at: NaiveDateTime,
}

#[impl_template]
impl From<EthereumBitcoinEtherBitcoinAcceptedSwap>
    for AcceptedSwap<
        Ethereum,
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        asset::Ether,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
{
    fn from(record: EthereumBitcoinEtherBitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                beta_ledger: __TYPE0__,
                alpha_asset: record.ether_amount.0.into(),
                beta_asset: record.bitcoin_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.bitcoin_redeem_identity.0.into(),
                alpha_expiry: record.ethereum_expiry.0.into(),
                beta_expiry: record.bitcoin_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.bitcoin_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[impl_template]
#[async_trait]
impl
    LoadAcceptedSwap<
        Ethereum,
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        asset::Ether,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    > for Sqlite
{
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<
            Ethereum,
            __TYPE0__,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > {
        use schema::{
            rfc003_ethereum_bitcoin_accept_messages as accept_messages,
            rfc003_ethereum_bitcoin_ether_bitcoin_request_messages as request_messages,
        };

        let record: EthereumBitcoinEtherBitcoinAcceptedSwap = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                request_messages::table
                    .inner_join(
                        accept_messages::table
                            .on(request_messages::swap_id.eq(accept_messages::swap_id)),
                    )
                    .select((
                        request_messages::swap_id,
                        request_messages::ethereum_chain_id,
                        request_messages::bitcoin_network,
                        request_messages::ether_amount,
                        request_messages::bitcoin_amount,
                        request_messages::hash_function,
                        request_messages::ethereum_refund_identity,
                        request_messages::bitcoin_redeem_identity,
                        request_messages::ethereum_expiry,
                        request_messages::bitcoin_expiry,
                        request_messages::secret_hash,
                        accept_messages::ethereum_redeem_identity,
                        accept_messages::bitcoin_refund_identity,
                        accept_messages::at,
                    ))
                    .filter(accept_messages::swap_id.eq(key))
                    .first(connection)
            })
            .await?;

        Ok(record.into())
    }
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct BitcoinEthereumBitcoinErc20AcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    bitcoin_network: Text<BitcoinNetwork>,
    ethereum_chain_id: U32,
    bitcoin_amount: Text<Satoshis>,
    erc20_token_contract: Text<EthereumAddress>,
    erc20_amount: Text<Erc20Amount>,
    hash_function: Text<HashFunction>,
    bitcoin_refund_identity: Text<::bitcoin::PublicKey>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    bitcoin_redeem_identity: Text<::bitcoin::PublicKey>,
    ethereum_refund_identity: Text<EthereumAddress>,

    at: NaiveDateTime,
}

#[impl_template]
impl From<BitcoinEthereumBitcoinErc20AcceptedSwap>
    for AcceptedSwap<
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        Ethereum,
        asset::Bitcoin,
        asset::Erc20,
        identity::Bitcoin,
        identity::Ethereum,
    >
{
    fn from(record: BitcoinEthereumBitcoinErc20AcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: __TYPE0__,
                beta_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                alpha_asset: record.bitcoin_amount.0.into(),
                beta_asset: asset::Erc20::new(
                    record.erc20_token_contract.0.into(),
                    record.erc20_amount.0.into(),
                ),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.bitcoin_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                alpha_expiry: record.bitcoin_expiry.0.into(),
                beta_expiry: record.ethereum_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.bitcoin_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[impl_template]
#[async_trait]
impl
    LoadAcceptedSwap<
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        Ethereum,
        asset::Bitcoin,
        asset::Erc20,
        identity::Bitcoin,
        identity::Ethereum,
    > for Sqlite
{
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<
            __TYPE0__,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > {
        use schema::{
            rfc003_bitcoin_ethereum_accept_messages as accept_messages,
            rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages as request_messages,
        };

        let record: BitcoinEthereumBitcoinErc20AcceptedSwap = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                request_messages::table
                    .inner_join(
                        accept_messages::table
                            .on(request_messages::swap_id.eq(accept_messages::swap_id)),
                    )
                    .select((
                        request_messages::swap_id,
                        request_messages::bitcoin_network,
                        request_messages::ethereum_chain_id,
                        request_messages::bitcoin_amount,
                        request_messages::erc20_token_contract,
                        request_messages::erc20_amount,
                        request_messages::hash_function,
                        request_messages::bitcoin_refund_identity,
                        request_messages::ethereum_redeem_identity,
                        request_messages::bitcoin_expiry,
                        request_messages::ethereum_expiry,
                        request_messages::secret_hash,
                        accept_messages::bitcoin_redeem_identity,
                        accept_messages::ethereum_refund_identity,
                        accept_messages::at,
                    ))
                    .filter(accept_messages::swap_id.eq(key))
                    .first(connection)
            })
            .await?;

        Ok(record.into())
    }
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct EthereumBitcoinErc20BitcoinAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: Text<BitcoinNetwork>,
    erc20_token_contract: Text<EthereumAddress>,
    erc20_amount: Text<Erc20Amount>,
    bitcoin_amount: Text<Satoshis>,
    hash_function: Text<HashFunction>,
    ethereum_refund_identity: Text<EthereumAddress>,
    bitcoin_redeem_identity: Text<::bitcoin::PublicKey>,
    ethereum_expiry: U32,
    bitcoin_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_refund_identity: Text<::bitcoin::PublicKey>,

    at: NaiveDateTime,
}

#[impl_template]
impl From<EthereumBitcoinErc20BitcoinAcceptedSwap>
    for AcceptedSwap<
        Ethereum,
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        asset::Erc20,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    >
{
    fn from(record: EthereumBitcoinErc20BitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                beta_ledger: __TYPE0__,
                alpha_asset: asset::Erc20::new(
                    record.erc20_token_contract.0.into(),
                    record.erc20_amount.0.into(),
                ),
                beta_asset: record.bitcoin_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.bitcoin_redeem_identity.0.into(),
                alpha_expiry: record.ethereum_expiry.0.into(),
                beta_expiry: record.bitcoin_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.bitcoin_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[impl_template]
#[async_trait]
impl
    LoadAcceptedSwap<
        Ethereum,
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        asset::Erc20,
        asset::Bitcoin,
        identity::Ethereum,
        identity::Bitcoin,
    > for Sqlite
{
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<
            Ethereum,
            __TYPE0__,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > {
        use schema::{
            rfc003_ethereum_bitcoin_accept_messages as accept_messages,
            rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages as request_messages,
        };

        let record: EthereumBitcoinErc20BitcoinAcceptedSwap = self
            .do_in_transaction(|connection| {
                let key = Text(key);

                request_messages::table
                    .inner_join(
                        accept_messages::table
                            .on(request_messages::swap_id.eq(accept_messages::swap_id)),
                    )
                    .select((
                        request_messages::swap_id,
                        request_messages::ethereum_chain_id,
                        request_messages::bitcoin_network,
                        request_messages::erc20_token_contract,
                        request_messages::erc20_amount,
                        request_messages::bitcoin_amount,
                        request_messages::hash_function,
                        request_messages::ethereum_refund_identity,
                        request_messages::bitcoin_redeem_identity,
                        request_messages::ethereum_expiry,
                        request_messages::bitcoin_expiry,
                        request_messages::secret_hash,
                        accept_messages::ethereum_redeem_identity,
                        accept_messages::bitcoin_refund_identity,
                        accept_messages::at,
                    ))
                    .filter(accept_messages::swap_id.eq(key))
                    .first(connection)
            })
            .await?;

        Ok(record.into())
    }
}
