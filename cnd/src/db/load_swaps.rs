use crate::db::{
    schema,
    wrapper_types::{
        custom_sql_types::{Text, U32},
        BitcoinNetwork, Erc20Amount, Ether, EthereumAddress, Satoshis,
    },
    Sqlite,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use comit::{
    asset::{self, Asset},
    bitcoin,
    swap_protocols::{
        ledger::{self, Ethereum},
        rfc003::{
            messages::{Accept, Request},
            AcceptedSwap, Ledger, SecretHash,
        },
        HashFunction, SwapId,
    },
};
use diesel::{self, prelude::*, RunQueryDsl};
use schema::{
    rfc003_bitcoin_ethereum_accept_messages,
    rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages,
    rfc003_bitcoin_ethereum_bitcoin_ether_request_messages,
    rfc003_ethereum_bitcoin_accept_messages,
    rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages,
    rfc003_ethereum_bitcoin_ether_bitcoin_request_messages,
};

#[async_trait]
pub trait LoadAcceptedSwap<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    async fn load_accepted_swap(
        &self,
        swap_id: &SwapId,
    ) -> anyhow::Result<AcceptedSwap<AL, BL, AA, BA>>;
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
    bitcoin_refund_identity: Text<bitcoin::PublicKey>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    bitcoin_redeem_identity: Text<bitcoin::PublicKey>,
    ethereum_refund_identity: Text<EthereumAddress>,

    at: NaiveDateTime,
}

impl From<BitcoinEthereumBitcoinEtherAcceptedSwap>
    for AcceptedSwap<ledger::bitcoin::Regtest, Ethereum, asset::Bitcoin, asset::Ether>
{
    fn from(record: BitcoinEthereumBitcoinEtherAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: ledger::bitcoin::Regtest::default(),
                beta_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                alpha_asset: record.bitcoin_amount.0.into(),
                beta_asset: record.ether_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.bitcoin_refund_identity.0,
                beta_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                alpha_expiry: record.bitcoin_expiry.into(),
                beta_expiry: record.ethereum_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                beta_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<ledger::bitcoin::Regtest, Ethereum, asset::Bitcoin, asset::Ether> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<ledger::bitcoin::Regtest, Ethereum, asset::Bitcoin, asset::Ether>,
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

impl From<BitcoinEthereumBitcoinEtherAcceptedSwap>
    for AcceptedSwap<ledger::bitcoin::Testnet, Ethereum, asset::Bitcoin, asset::Ether>
{
    fn from(record: BitcoinEthereumBitcoinEtherAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: ledger::bitcoin::Testnet::default(),
                beta_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                alpha_asset: record.bitcoin_amount.0.into(),
                beta_asset: record.ether_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.bitcoin_refund_identity.0,
                beta_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                alpha_expiry: record.bitcoin_expiry.into(),
                beta_expiry: record.ethereum_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                beta_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<ledger::bitcoin::Testnet, Ethereum, asset::Bitcoin, asset::Ether> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<ledger::bitcoin::Testnet, Ethereum, asset::Bitcoin, asset::Ether>,
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

impl From<BitcoinEthereumBitcoinEtherAcceptedSwap>
    for AcceptedSwap<ledger::bitcoin::Mainnet, Ethereum, asset::Bitcoin, asset::Ether>
{
    fn from(record: BitcoinEthereumBitcoinEtherAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: ledger::bitcoin::Mainnet::default(),
                beta_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                alpha_asset: record.bitcoin_amount.0.into(),
                beta_asset: record.ether_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.bitcoin_refund_identity.0,
                beta_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                alpha_expiry: record.bitcoin_expiry.into(),
                beta_expiry: record.ethereum_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                beta_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<ledger::bitcoin::Mainnet, Ethereum, asset::Bitcoin, asset::Ether> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<ledger::bitcoin::Mainnet, Ethereum, asset::Bitcoin, asset::Ether>,
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
    bitcoin_redeem_identity: Text<bitcoin::PublicKey>,
    ethereum_expiry: U32,
    bitcoin_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_refund_identity: Text<bitcoin::PublicKey>,

    at: NaiveDateTime,
}

impl From<EthereumBitcoinEtherBitcoinAcceptedSwap>
    for AcceptedSwap<Ethereum, ledger::bitcoin::Regtest, asset::Ether, asset::Bitcoin>
{
    fn from(record: EthereumBitcoinEtherBitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                beta_ledger: ledger::bitcoin::Regtest::default(),
                alpha_asset: record.ether_amount.0.into(),
                beta_asset: record.bitcoin_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                alpha_expiry: record.ethereum_expiry.0.into(),
                beta_expiry: record.bitcoin_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.bitcoin_refund_identity.0,
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<Ethereum, ledger::bitcoin::Regtest, asset::Ether, asset::Bitcoin> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Ethereum, ledger::bitcoin::Regtest, asset::Ether, asset::Bitcoin>,
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

impl From<EthereumBitcoinEtherBitcoinAcceptedSwap>
    for AcceptedSwap<Ethereum, ledger::bitcoin::Testnet, asset::Ether, asset::Bitcoin>
{
    fn from(record: EthereumBitcoinEtherBitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                beta_ledger: ledger::bitcoin::Testnet::default(),
                alpha_asset: record.ether_amount.0.into(),
                beta_asset: record.bitcoin_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                alpha_expiry: record.ethereum_expiry.0.into(),
                beta_expiry: record.bitcoin_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.bitcoin_refund_identity.0,
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<Ethereum, ledger::bitcoin::Testnet, asset::Ether, asset::Bitcoin> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Ethereum, ledger::bitcoin::Testnet, asset::Ether, asset::Bitcoin>,
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

impl From<EthereumBitcoinEtherBitcoinAcceptedSwap>
    for AcceptedSwap<Ethereum, ledger::bitcoin::Mainnet, asset::Ether, asset::Bitcoin>
{
    fn from(record: EthereumBitcoinEtherBitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                beta_ledger: ledger::bitcoin::Mainnet::default(),
                alpha_asset: record.ether_amount.0.into(),
                beta_asset: record.bitcoin_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                alpha_expiry: record.ethereum_expiry.0.into(),
                beta_expiry: record.bitcoin_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.bitcoin_refund_identity.0,
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<Ethereum, ledger::bitcoin::Mainnet, asset::Ether, asset::Bitcoin> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Ethereum, ledger::bitcoin::Mainnet, asset::Ether, asset::Bitcoin>,
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
    bitcoin_refund_identity: Text<bitcoin::PublicKey>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    bitcoin_redeem_identity: Text<bitcoin::PublicKey>,
    ethereum_refund_identity: Text<EthereumAddress>,

    at: NaiveDateTime,
}

impl From<BitcoinEthereumBitcoinErc20AcceptedSwap>
    for AcceptedSwap<ledger::bitcoin::Regtest, Ethereum, asset::Bitcoin, asset::Erc20>
{
    fn from(record: BitcoinEthereumBitcoinErc20AcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: ledger::bitcoin::Regtest::default(),
                beta_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                alpha_asset: record.bitcoin_amount.0.into(),
                beta_asset: asset::Erc20::new(
                    record.erc20_token_contract.0.into(),
                    record.erc20_amount.0.into(),
                ),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.bitcoin_refund_identity.0,
                beta_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                alpha_expiry: record.bitcoin_expiry.0.into(),
                beta_expiry: record.ethereum_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                beta_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<ledger::bitcoin::Regtest, Ethereum, asset::Bitcoin, asset::Erc20> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<ledger::bitcoin::Regtest, Ethereum, asset::Bitcoin, asset::Erc20>,
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

impl From<BitcoinEthereumBitcoinErc20AcceptedSwap>
    for AcceptedSwap<ledger::bitcoin::Testnet, Ethereum, asset::Bitcoin, asset::Erc20>
{
    fn from(record: BitcoinEthereumBitcoinErc20AcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: ledger::bitcoin::Testnet::default(),
                beta_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                alpha_asset: record.bitcoin_amount.0.into(),
                beta_asset: asset::Erc20::new(
                    record.erc20_token_contract.0.into(),
                    record.erc20_amount.0.into(),
                ),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.bitcoin_refund_identity.0,
                beta_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                alpha_expiry: record.bitcoin_expiry.0.into(),
                beta_expiry: record.ethereum_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                beta_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<ledger::bitcoin::Testnet, Ethereum, asset::Bitcoin, asset::Erc20> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<ledger::bitcoin::Testnet, Ethereum, asset::Bitcoin, asset::Erc20>,
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

impl From<BitcoinEthereumBitcoinErc20AcceptedSwap>
    for AcceptedSwap<ledger::bitcoin::Mainnet, Ethereum, asset::Bitcoin, asset::Erc20>
{
    fn from(record: BitcoinEthereumBitcoinErc20AcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: ledger::bitcoin::Mainnet::default(),
                beta_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                alpha_asset: record.bitcoin_amount.0.into(),
                beta_asset: asset::Erc20::new(
                    record.erc20_token_contract.0.into(),
                    record.erc20_amount.0.into(),
                ),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.bitcoin_refund_identity.0,
                beta_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                alpha_expiry: record.bitcoin_expiry.0.into(),
                beta_expiry: record.ethereum_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                beta_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<ledger::bitcoin::Mainnet, Ethereum, asset::Bitcoin, asset::Erc20> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<ledger::bitcoin::Mainnet, Ethereum, asset::Bitcoin, asset::Erc20>,
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
    bitcoin_redeem_identity: Text<bitcoin::PublicKey>,
    ethereum_expiry: U32,
    bitcoin_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_refund_identity: Text<bitcoin::PublicKey>,

    at: NaiveDateTime,
}

impl From<EthereumBitcoinErc20BitcoinAcceptedSwap>
    for AcceptedSwap<Ethereum, ledger::bitcoin::Regtest, asset::Erc20, asset::Bitcoin>
{
    fn from(record: EthereumBitcoinErc20BitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                beta_ledger: ledger::bitcoin::Regtest::default(),
                alpha_asset: asset::Erc20::new(
                    record.erc20_token_contract.0.into(),
                    record.erc20_amount.0.into(),
                ),
                beta_asset: record.bitcoin_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                alpha_expiry: record.ethereum_expiry.0.into(),
                beta_expiry: record.bitcoin_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.bitcoin_refund_identity.0,
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<Ethereum, ledger::bitcoin::Regtest, asset::Erc20, asset::Bitcoin> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Ethereum, ledger::bitcoin::Regtest, asset::Erc20, asset::Bitcoin>,
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

impl From<EthereumBitcoinErc20BitcoinAcceptedSwap>
    for AcceptedSwap<Ethereum, ledger::bitcoin::Testnet, asset::Erc20, asset::Bitcoin>
{
    fn from(record: EthereumBitcoinErc20BitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                beta_ledger: ledger::bitcoin::Testnet::default(),
                alpha_asset: asset::Erc20::new(
                    record.erc20_token_contract.0.into(),
                    record.erc20_amount.0.into(),
                ),
                beta_asset: record.bitcoin_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                alpha_expiry: record.ethereum_expiry.0.into(),
                beta_expiry: record.bitcoin_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.bitcoin_refund_identity.0,
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<Ethereum, ledger::bitcoin::Testnet, asset::Erc20, asset::Bitcoin> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Ethereum, ledger::bitcoin::Testnet, asset::Erc20, asset::Bitcoin>,
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

impl From<EthereumBitcoinErc20BitcoinAcceptedSwap>
    for AcceptedSwap<Ethereum, ledger::bitcoin::Mainnet, asset::Erc20, asset::Bitcoin>
{
    fn from(record: EthereumBitcoinErc20BitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: record.ethereum_chain_id.0.into(),
                },
                beta_ledger: ledger::bitcoin::Mainnet::default(),
                alpha_asset: asset::Erc20::new(
                    record.erc20_token_contract.0.into(),
                    record.erc20_amount.0.into(),
                ),
                beta_asset: record.bitcoin_amount.0.into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: record.ethereum_refund_identity.0.into(),
                beta_ledger_redeem_identity: record.bitcoin_redeem_identity.0,
                alpha_expiry: record.ethereum_expiry.0.into(),
                beta_expiry: record.bitcoin_expiry.0.into(),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: record.ethereum_redeem_identity.0.into(),
                beta_ledger_refund_identity: record.bitcoin_refund_identity.0,
            },
            record.at,
        )
    }
}

#[async_trait]
impl LoadAcceptedSwap<Ethereum, ledger::bitcoin::Mainnet, asset::Erc20, asset::Bitcoin> for Sqlite {
    async fn load_accepted_swap(
        &self,
        key: &SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Ethereum, ledger::bitcoin::Mainnet, asset::Erc20, asset::Bitcoin>,
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
