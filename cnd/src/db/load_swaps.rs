use crate::{
    db::{
        custom_sql_types::{Text, U32},
        new_types::{DecimalU256, EthereumAddress, Satoshis},
        schema, Sqlite,
    },
    swap_protocols::{
        asset::Asset,
        ledger::{ethereum::ChainId, Bitcoin, Ethereum},
        rfc003::{
            messages::{Accept, Request},
            Ledger, SecretHash,
        },
        HashFunction, SwapId, Timestamp,
    },
};
use diesel::{self, prelude::*, RunQueryDsl};
use ethereum_support::{Erc20Quantity, Erc20Token, EtherQuantity, U256};

pub type AcceptedSwap<AL, BL, AA, BA> = (Request<AL, BL, AA, BA>, Accept<AL, BL>);

pub trait LoadAcceptedSwap<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    fn load_accepted_swap(&self, swap_id: SwapId) -> anyhow::Result<AcceptedSwap<AL, BL, AA, BA>>;
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct BitcoinEthereumBitcoinEtherAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    bitcoin_network: Text<bitcoin::Network>,
    ethereum_chain_id: U32,
    bitcoin_amount: Text<Satoshis>,
    ether_amount: Text<DecimalU256>,
    hash_function: Text<HashFunction>,
    bitcoin_refund_identity: Text<bitcoin::PublicKey>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    bitcoin_redeem_identity: Text<bitcoin::PublicKey>,
    ethereum_refund_identity: Text<EthereumAddress>,
}

impl LoadAcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, EtherQuantity> for Sqlite {
    fn load_accepted_swap(
        &self,
        key: SwapId,
    ) -> anyhow::Result<
        AcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::EtherQuantity>,
    > {
        use schema::{
            rfc003_bitcoin_ethereum_accept_messages as accept_messages,
            rfc003_bitcoin_ethereum_bitcoin_ether_request_messages as request_messages,
        };

        diesel::allow_tables_to_appear_in_same_query!(request_messages, accept_messages);

        let connection = self.connect()?;
        let key = Text(key);

        let record: BitcoinEthereumBitcoinEtherAcceptedSwap = request_messages::table
            .inner_join(
                accept_messages::table.on(request_messages::swap_id.eq(accept_messages::swap_id)),
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
            ))
            .filter(accept_messages::swap_id.eq(key))
            .first(&connection)?;

        Ok((
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Bitcoin {
                    network: *record.bitcoin_network,
                },
                beta_ledger: Ethereum {
                    chain_id: ChainId::new(record.ethereum_chain_id.into()),
                },
                alpha_asset: bitcoin::Amount::from_sat(u64::from(*record.bitcoin_amount)),
                beta_asset: EtherQuantity::from_wei(U256::from(*record.ether_amount)),
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
        ))
    }
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct EthereumBitcoinEtherBitcoinAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: Text<bitcoin::Network>,
    ether_amount: Text<DecimalU256>,
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
}

impl LoadAcceptedSwap<Ethereum, Bitcoin, EtherQuantity, bitcoin::Amount> for Sqlite {
    fn load_accepted_swap(
        &self,
        key: SwapId,
    ) -> anyhow::Result<AcceptedSwap<Ethereum, Bitcoin, EtherQuantity, bitcoin::Amount>> {
        use schema::{
            rfc003_ethereum_bitcoin_accept_messages as accept_messages,
            rfc003_ethereum_bitcoin_ether_bitcoin_request_messages as request_messages,
        };

        diesel::allow_tables_to_appear_in_same_query!(request_messages, accept_messages);

        let connection = self.connect()?;
        let key = Text(key);

        let record: EthereumBitcoinEtherBitcoinAcceptedSwap = request_messages::table
            .inner_join(
                accept_messages::table.on(request_messages::swap_id.eq(accept_messages::swap_id)),
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
            ))
            .filter(accept_messages::swap_id.eq(key))
            .first(&connection)?;

        Ok((
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: ChainId::new(record.ethereum_chain_id.into()),
                },
                beta_ledger: Bitcoin {
                    network: *record.bitcoin_network,
                },
                alpha_asset: EtherQuantity::from_wei(U256::from(*record.ether_amount)),
                beta_asset: bitcoin::Amount::from_sat(u64::from(*record.bitcoin_amount)),
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
        ))
    }
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct BitcoinEthereumBitcoinErc20AcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    bitcoin_network: Text<bitcoin::Network>,
    ethereum_chain_id: U32,
    bitcoin_amount: Text<Satoshis>,
    erc20_token_contract: Text<EthereumAddress>,
    erc20_amount: Text<DecimalU256>,
    hash_function: Text<HashFunction>,
    bitcoin_refund_identity: Text<bitcoin::PublicKey>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: Text<SecretHash>,
    // Accept fields.
    bitcoin_redeem_identity: Text<bitcoin::PublicKey>,
    ethereum_refund_identity: Text<EthereumAddress>,
}

impl LoadAcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, Erc20Token> for Sqlite {
    fn load_accepted_swap(
        &self,
        key: SwapId,
    ) -> anyhow::Result<AcceptedSwap<Bitcoin, Ethereum, bitcoin::Amount, Erc20Token>> {
        use schema::{
            rfc003_bitcoin_ethereum_accept_messages as accept_messages,
            rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages as request_messages,
        };

        diesel::allow_tables_to_appear_in_same_query!(request_messages, accept_messages);

        let connection = self.connect()?;
        let key = Text(key);

        let record: BitcoinEthereumBitcoinErc20AcceptedSwap = request_messages::table
            .inner_join(
                accept_messages::table.on(request_messages::swap_id.eq(accept_messages::swap_id)),
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
            ))
            .filter(accept_messages::swap_id.eq(key))
            .first(&connection)?;

        Ok((
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Bitcoin {
                    network: *record.bitcoin_network,
                },
                beta_ledger: Ethereum {
                    chain_id: ChainId::new(record.ethereum_chain_id.into()),
                },
                alpha_asset: bitcoin::Amount::from_sat(u64::from(*record.bitcoin_amount)),
                beta_asset: Erc20Token::new(
                    (record.erc20_token_contract.0).0,
                    Erc20Quantity((record.erc20_amount.0).0),
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
        ))
    }
}

#[derive(Queryable, Debug, Clone, PartialEq)]
struct EthereumBitcoinErc20BitcoinAcceptedSwap {
    // Request fields.
    swap_id: Text<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: Text<bitcoin::Network>,
    erc20_token_contract: Text<EthereumAddress>,
    erc20_amount: Text<DecimalU256>,
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
}

impl LoadAcceptedSwap<Ethereum, Bitcoin, Erc20Token, bitcoin::Amount> for Sqlite {
    fn load_accepted_swap(
        &self,
        key: SwapId,
    ) -> anyhow::Result<AcceptedSwap<Ethereum, Bitcoin, Erc20Token, bitcoin::Amount>> {
        use schema::{
            rfc003_ethereum_bitcoin_accept_messages as accept_messages,
            rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages as request_messages,
        };

        diesel::allow_tables_to_appear_in_same_query!(request_messages, accept_messages);

        let connection = self.connect()?;
        let key = Text(key);

        let record: EthereumBitcoinErc20BitcoinAcceptedSwap = request_messages::table
            .inner_join(
                accept_messages::table.on(request_messages::swap_id.eq(accept_messages::swap_id)),
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
            ))
            .filter(accept_messages::swap_id.eq(key))
            .first(&connection)?;

        Ok((
            Request {
                swap_id: *record.swap_id,
                alpha_ledger: Ethereum {
                    chain_id: ChainId::new(record.ethereum_chain_id.into()),
                },
                beta_ledger: Bitcoin {
                    network: *record.bitcoin_network,
                },
                alpha_asset: Erc20Token::new(
                    (record.erc20_token_contract.0).0,
                    Erc20Quantity((record.erc20_amount.0).0),
                ),
                beta_asset: bitcoin::Amount::from_sat(u64::from(*record.bitcoin_amount)),
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
        ))
    }
}
