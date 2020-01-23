use crate::{
    asset::{self, Asset},
    db::{
        custom_sql_types::{Text, U32},
        new_types::{Erc20Amount, Ether, EthereumAddress, Satoshis},
        schema, Sqlite,
    },
    swap_protocols::{
        ledger::{ethereum::ChainId, Bitcoin, Ethereum},
        rfc003::{
            messages::{Accept, Request},
            Ledger, SecretHash,
        },
        HashFunction, SwapId,
    },
    timestamp::Timestamp,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::{self, prelude::*, RunQueryDsl};

pub type AcceptedSwap<AL, BL, AA, BA> = (Request<AL, BL, AA, BA>, Accept<AL, BL>, NaiveDateTime);

#[async_trait]
pub trait LoadAcceptedSwap<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    async fn load_accepted_swap(
        &self,
        swap_id: &SwapId,
    ) -> anyhow::Result<AcceptedSwap<AL, BL, AA, BA>>;
}

macro_rules! impl_load_accepted_swap {
    ($al:tt, $bl:tt, $aa:tt, $ba:tt, $accept_messages:tt, $request_messages:tt,
    $record:tt, $select:tt) => {
        paste::item! {
            // Parenthesis are needed because we use `tt`
            // because of https://github.com/dtolnay/async-trait/issues/46
            #[allow(unused_parens)]
            #[async_trait]
            impl LoadAcceptedSwap<$al, $bl, $aa, $ba> for Sqlite {
                async fn load_accepted_swap(
                    &self,
                    key: &SwapId,
                ) -> anyhow::Result<AcceptedSwap<$al, $bl, $aa, $ba>> {
                    use schema::{
                         $accept_messages as accept_messages,
                         $request_messages as request_messages,
                    };

                    diesel::allow_tables_to_appear_in_same_query!(request_messages, accept_messages);

                    let record: $record = self
                        .do_in_transaction(|connection| {
                            let key = Text(key);

                            request_messages::table
                                .inner_join(
                                    accept_messages::table
                                        .on(request_messages::swap_id.eq(accept_messages::swap_id)),
                                )
                                .select($select)
                                .filter(accept_messages::swap_id.eq(key))
                                .first(connection)
                        })
                        .await?;

                    Ok(record.into())
                }
        }
        }
    };
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct BitcoinEthereumBitcoinEtherAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    bitcoin_network: Text<bitcoin::Network>,
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
    for AcceptedSwap<Bitcoin, Ethereum, asset::Bitcoin, asset::Ether>
{
    fn from(record: BitcoinEthereumBitcoinEtherAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Bitcoin {
                    network: *record.bitcoin_network,
                },
                beta_ledger: Ethereum {
                    chain_id: ChainId::new(record.ethereum_chain_id.into()),
                },
                alpha_asset: asset::Bitcoin::from_sat(u64::from(*record.bitcoin_amount)),
                beta_asset: (record.ether_amount.0).into(),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: crate::bitcoin::PublicKey::from(
                    *record.bitcoin_refund_identity,
                ),
                beta_ledger_redeem_identity: (record.ethereum_redeem_identity.0).0,
                alpha_expiry: Timestamp::from(u32::from(record.bitcoin_expiry)),
                beta_expiry: Timestamp::from(u32::from(record.ethereum_expiry)),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: crate::bitcoin::PublicKey::from(
                    *record.bitcoin_redeem_identity,
                ),
                beta_ledger_refund_identity: (record.ethereum_refund_identity.0).0,
            },
            record.at,
        )
    }
}

impl_load_accepted_swap!(
    Bitcoin,
    Ethereum,
    (asset::Bitcoin),
    (asset::Ether),
    rfc003_bitcoin_ethereum_accept_messages,
    rfc003_bitcoin_ethereum_bitcoin_ether_request_messages,
    BitcoinEthereumBitcoinEtherAcceptedSwap,
    (
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
    )
);

#[derive(Queryable, Debug, Clone, PartialEq)]
struct EthereumBitcoinEtherBitcoinAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: Text<bitcoin::Network>,
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
    for AcceptedSwap<Ethereum, Bitcoin, asset::Ether, asset::Bitcoin>
{
    fn from(record: EthereumBitcoinEtherBitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: ChainId::new(record.ethereum_chain_id.into()),
                },
                beta_ledger: Bitcoin {
                    network: *record.bitcoin_network,
                },
                alpha_asset: (record.ether_amount.0).into(),
                beta_asset: asset::Bitcoin::from_sat(u64::from(*record.bitcoin_amount)),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: (record.ethereum_refund_identity.0).0,
                beta_ledger_redeem_identity: crate::bitcoin::PublicKey::from(
                    *record.bitcoin_redeem_identity,
                ),
                alpha_expiry: Timestamp::from(u32::from(record.ethereum_expiry)),
                beta_expiry: Timestamp::from(u32::from(record.bitcoin_expiry)),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: (record.ethereum_redeem_identity.0).0,
                beta_ledger_refund_identity: crate::bitcoin::PublicKey::from(
                    *record.bitcoin_refund_identity,
                ),
            },
            record.at,
        )
    }
}

impl_load_accepted_swap!(
    Ethereum,
    Bitcoin,
    (asset::Ether),
    (asset::Bitcoin),
    rfc003_ethereum_bitcoin_accept_messages,
    rfc003_ethereum_bitcoin_ether_bitcoin_request_messages,
    EthereumBitcoinEtherBitcoinAcceptedSwap,
    (
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
    )
);

#[derive(Queryable, Debug, Clone, PartialEq)]
struct BitcoinEthereumBitcoinErc20AcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    bitcoin_network: Text<bitcoin::Network>,
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
    for AcceptedSwap<Bitcoin, Ethereum, asset::Bitcoin, asset::Erc20>
{
    fn from(record: BitcoinEthereumBitcoinErc20AcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Bitcoin {
                    network: *record.bitcoin_network,
                },
                beta_ledger: Ethereum {
                    chain_id: ChainId::new(record.ethereum_chain_id.into()),
                },
                alpha_asset: asset::Bitcoin::from_sat(u64::from(*record.bitcoin_amount)),
                beta_asset: asset::Erc20::new(
                    (record.erc20_token_contract.0).0,
                    (record.erc20_amount.0).into(),
                ),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: crate::bitcoin::PublicKey::from(
                    *record.bitcoin_refund_identity,
                ),
                beta_ledger_redeem_identity: (record.ethereum_redeem_identity.0).0,
                alpha_expiry: Timestamp::from(u32::from(record.bitcoin_expiry)),
                beta_expiry: Timestamp::from(u32::from(record.ethereum_expiry)),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: crate::bitcoin::PublicKey::from(
                    *record.bitcoin_redeem_identity,
                ),
                beta_ledger_refund_identity: (record.ethereum_refund_identity.0).0,
            },
            record.at,
        )
    }
}

impl_load_accepted_swap!(
    Bitcoin,
    Ethereum,
    (asset::Bitcoin),
    (asset::Erc20),
    rfc003_bitcoin_ethereum_accept_messages,
    rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages,
    BitcoinEthereumBitcoinErc20AcceptedSwap,
    (
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
    )
);

#[derive(Queryable, Debug, Clone, PartialEq)]
struct EthereumBitcoinErc20BitcoinAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: Text<bitcoin::Network>,
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
    for AcceptedSwap<Ethereum, Bitcoin, asset::Erc20, asset::Bitcoin>
{
    fn from(record: EthereumBitcoinErc20BitcoinAcceptedSwap) -> Self {
        (
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: ChainId::new(record.ethereum_chain_id.into()),
                },
                beta_ledger: Bitcoin {
                    network: *record.bitcoin_network,
                },
                alpha_asset: asset::Erc20::new(
                    (record.erc20_token_contract.0).0,
                    (record.erc20_amount.0).into(),
                ),
                beta_asset: asset::Bitcoin::from_sat(u64::from(*record.bitcoin_amount)),
                hash_function: *record.hash_function,
                alpha_ledger_refund_identity: (record.ethereum_refund_identity.0).0,
                beta_ledger_redeem_identity: crate::bitcoin::PublicKey::from(
                    *record.bitcoin_redeem_identity,
                ),
                alpha_expiry: Timestamp::from(u32::from(record.ethereum_expiry)),
                beta_expiry: Timestamp::from(u32::from(record.bitcoin_expiry)),
                secret_hash: *record.secret_hash,
            },
            Accept {
                swap_id: *record.swap_id,
                alpha_ledger_redeem_identity: (record.ethereum_redeem_identity.0).0,
                beta_ledger_refund_identity: crate::bitcoin::PublicKey::from(
                    *record.bitcoin_refund_identity,
                ),
            },
            record.at,
        )
    }
}

impl_load_accepted_swap!(
    Ethereum,
    Bitcoin,
    (asset::Erc20),
    (asset::Bitcoin),
    rfc003_ethereum_bitcoin_accept_messages,
    rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages,
    EthereumBitcoinErc20BitcoinAcceptedSwap,
    (
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
    )
);
