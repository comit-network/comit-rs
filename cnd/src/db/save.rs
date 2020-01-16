use crate::{
    asset,
    db::{
        custom_sql_types::{Text, U32},
        new_types::{DecimalU256, EthereumAddress, Satoshis},
        schema::{self, *},
        Sqlite, Swap,
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{Accept, Decline, Request, SecretHash},
        HashFunction, Role, SwapId,
    },
};
use async_trait::async_trait;
use diesel::RunQueryDsl;
use libp2p::{self, PeerId};

/// Save swap to database.
#[async_trait]
pub trait Save<T>: Send + Sync + 'static {
    async fn save(&self, swap: T) -> anyhow::Result<()>;
}

#[async_trait]
impl Save<Swap> for Sqlite {
    async fn save(&self, swap: Swap) -> anyhow::Result<()> {
        let insertable = InsertableSwap::from(swap);

        self.do_in_transaction(|connection| {
            diesel::insert_into(schema::rfc003_swaps::dsl::rfc003_swaps)
                .values(&insertable)
                .execute(&*connection)
        })
        .await?;

        Ok(())
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_swaps"]
struct InsertableSwap {
    pub swap_id: Text<SwapId>,
    pub role: Text<Role>,
    pub counterparty: Text<PeerId>,
}

impl From<Swap> for InsertableSwap {
    fn from(swap: Swap) -> Self {
        InsertableSwap {
            swap_id: Text(swap.swap_id),
            role: Text(swap.role),
            counterparty: Text(swap.counterparty),
        }
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_bitcoin_ethereum_bitcoin_ether_request_messages"]
struct InsertableBitcoinEthereumBitcoinEtherRequestMessage {
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
}

#[async_trait]
impl Save<Request<Bitcoin, Ethereum, bitcoin::Amount, asset::Ether>> for Sqlite {
    async fn save(
        &self,
        message: Request<Bitcoin, Ethereum, bitcoin::Amount, asset::Ether>,
    ) -> anyhow::Result<()> {
        let Request {
            swap_id,
            alpha_ledger,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
        } = message;

        let insertable = InsertableBitcoinEthereumBitcoinEtherRequestMessage {
            swap_id: Text(swap_id),
            bitcoin_network: Text(alpha_ledger.network),
            ethereum_chain_id: U32(beta_ledger.chain_id.into()),
            bitcoin_amount: Text(Satoshis(alpha_asset.as_sat())),
            ether_amount: Text(DecimalU256(beta_asset.wei())),
            hash_function: Text(hash_function),
            bitcoin_refund_identity: Text(alpha_ledger_refund_identity.into_inner()),
            ethereum_redeem_identity: Text(EthereumAddress(beta_ledger_redeem_identity)),
            bitcoin_expiry: U32(alpha_expiry.into()),
            ethereum_expiry: U32(beta_expiry.into()),
            secret_hash: Text(secret_hash),
        };

        self.do_in_transaction(|connection| {
            diesel::insert_into(rfc003_bitcoin_ethereum_bitcoin_ether_request_messages::table)
                .values(&insertable)
                .execute(connection)
        })
        .await?;

        Ok(())
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages"]
struct InsertableBitcoinEthereumBitcoinErc20RequestMessage {
    swap_id: Text<SwapId>,
    bitcoin_network: Text<bitcoin::Network>,
    ethereum_chain_id: U32,
    bitcoin_amount: Text<Satoshis>,
    erc20_amount: Text<DecimalU256>,
    erc20_token_contract: Text<EthereumAddress>,
    hash_function: Text<HashFunction>,
    bitcoin_refund_identity: Text<bitcoin::PublicKey>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: Text<SecretHash>,
}

#[async_trait]
impl Save<Request<Bitcoin, Ethereum, bitcoin::Amount, asset::Erc20>> for Sqlite {
    async fn save(
        &self,
        message: Request<Bitcoin, Ethereum, bitcoin::Amount, asset::Erc20>,
    ) -> anyhow::Result<()> {
        let Request {
            swap_id,
            alpha_ledger,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
        } = message;

        let insertable = InsertableBitcoinEthereumBitcoinErc20RequestMessage {
            swap_id: Text(swap_id),
            bitcoin_network: Text(alpha_ledger.network),
            ethereum_chain_id: U32(beta_ledger.chain_id.into()),
            bitcoin_amount: Text(Satoshis(alpha_asset.as_sat())),
            erc20_amount: Text(DecimalU256(beta_asset.quantity.0)),
            erc20_token_contract: Text(EthereumAddress(beta_asset.token_contract)),
            hash_function: Text(hash_function),
            bitcoin_refund_identity: Text(alpha_ledger_refund_identity.into_inner()),
            ethereum_redeem_identity: Text(EthereumAddress(beta_ledger_redeem_identity)),
            bitcoin_expiry: U32(alpha_expiry.into()),
            ethereum_expiry: U32(beta_expiry.into()),
            secret_hash: Text(secret_hash),
        };

        self.do_in_transaction(|connection| {
            diesel::insert_into(rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages::table)
                .values(&insertable)
                .execute(connection)
        })
        .await?;

        Ok(())
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_ether_bitcoin_request_messages"]
struct InsertableEthereumBitcoinEtherBitcoinRequestMessage {
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
}

#[async_trait]
impl Save<Request<Ethereum, Bitcoin, asset::Ether, bitcoin::Amount>> for Sqlite {
    async fn save(
        &self,
        message: Request<Ethereum, Bitcoin, asset::Ether, bitcoin::Amount>,
    ) -> anyhow::Result<()> {
        let Request {
            swap_id,
            alpha_ledger,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
        } = message;

        let insertable = InsertableEthereumBitcoinEtherBitcoinRequestMessage {
            swap_id: Text(swap_id),
            ethereum_chain_id: U32(alpha_ledger.chain_id.into()),
            bitcoin_network: Text(beta_ledger.network),
            ether_amount: Text(DecimalU256(alpha_asset.wei())),
            bitcoin_amount: Text(Satoshis(beta_asset.as_sat())),
            hash_function: Text(hash_function),
            ethereum_refund_identity: Text(EthereumAddress(alpha_ledger_refund_identity)),
            bitcoin_redeem_identity: Text(beta_ledger_redeem_identity.into_inner()),
            ethereum_expiry: U32(alpha_expiry.into()),
            bitcoin_expiry: U32(beta_expiry.into()),
            secret_hash: Text(secret_hash),
        };

        self.do_in_transaction(|connection| {
            diesel::insert_into(rfc003_ethereum_bitcoin_ether_bitcoin_request_messages::table)
                .values(&insertable)
                .execute(connection)
        })
        .await?;

        Ok(())
    }
}
#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages"]
struct InsertableEthereumBitcoinErc20BitcoinRequestMessage {
    swap_id: Text<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: Text<bitcoin::Network>,
    erc20_amount: Text<DecimalU256>,
    erc20_token_contract: Text<EthereumAddress>,
    bitcoin_amount: Text<Satoshis>,
    hash_function: Text<HashFunction>,
    ethereum_refund_identity: Text<EthereumAddress>,
    bitcoin_redeem_identity: Text<bitcoin::PublicKey>,
    ethereum_expiry: U32,
    bitcoin_expiry: U32,
    secret_hash: Text<SecretHash>,
}

#[async_trait]
impl Save<Request<Ethereum, Bitcoin, asset::Erc20, bitcoin::Amount>> for Sqlite {
    async fn save(
        &self,
        message: Request<Ethereum, Bitcoin, asset::Erc20, bitcoin::Amount>,
    ) -> anyhow::Result<()> {
        let Request {
            swap_id,
            alpha_ledger,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
        } = message;

        let insertable = InsertableEthereumBitcoinErc20BitcoinRequestMessage {
            swap_id: Text(swap_id),
            ethereum_chain_id: U32(alpha_ledger.chain_id.into()),
            bitcoin_network: Text(beta_ledger.network),
            erc20_amount: Text(DecimalU256(alpha_asset.quantity.0)),
            erc20_token_contract: Text(EthereumAddress(alpha_asset.token_contract)),
            bitcoin_amount: Text(Satoshis(beta_asset.as_sat())),
            hash_function: Text(hash_function),
            ethereum_refund_identity: Text(EthereumAddress(alpha_ledger_refund_identity)),
            bitcoin_redeem_identity: Text(beta_ledger_redeem_identity.into_inner()),
            ethereum_expiry: U32(alpha_expiry.into()),
            bitcoin_expiry: U32(beta_expiry.into()),
            secret_hash: Text(secret_hash),
        };

        self.do_in_transaction(|connection| {
            diesel::insert_into(rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages::table)
                .values(&insertable)
                .execute(connection)
        })
        .await?;

        Ok(())
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_accept_messages"]
struct InsertableEthereumBitcoinAcceptMessage {
    swap_id: Text<SwapId>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_refund_identity: Text<bitcoin::PublicKey>,
}

#[async_trait]
impl Save<Accept<Ethereum, Bitcoin>> for Sqlite {
    async fn save(&self, message: Accept<Ethereum, Bitcoin>) -> anyhow::Result<()> {
        let Accept {
            swap_id,
            alpha_ledger_redeem_identity,
            beta_ledger_refund_identity,
        } = message;

        let insertable = InsertableEthereumBitcoinAcceptMessage {
            swap_id: Text(swap_id),
            ethereum_redeem_identity: Text(EthereumAddress(alpha_ledger_redeem_identity)),
            bitcoin_refund_identity: Text(beta_ledger_refund_identity.into_inner()),
        };

        self.do_in_transaction(|connection| {
            diesel::insert_into(rfc003_ethereum_bitcoin_accept_messages::table)
                .values(&insertable)
                .execute(&*connection)
        })
        .await?;

        Ok(())
    }
}
#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_bitcoin_ethereum_accept_messages"]
struct InsertableBitcoinEthereumAcceptMessage {
    swap_id: Text<SwapId>,
    bitcoin_redeem_identity: Text<bitcoin::PublicKey>,
    ethereum_refund_identity: Text<EthereumAddress>,
}

#[async_trait]
impl Save<Accept<Bitcoin, Ethereum>> for Sqlite {
    async fn save(&self, message: Accept<Bitcoin, Ethereum>) -> anyhow::Result<()> {
        let Accept {
            swap_id,
            alpha_ledger_redeem_identity,
            beta_ledger_refund_identity,
        } = message;

        let insertable = InsertableBitcoinEthereumAcceptMessage {
            swap_id: Text(swap_id),
            bitcoin_redeem_identity: Text(alpha_ledger_redeem_identity.into_inner()),
            ethereum_refund_identity: Text(EthereumAddress(beta_ledger_refund_identity)),
        };

        self.do_in_transaction(|connection| {
            diesel::insert_into(rfc003_bitcoin_ethereum_accept_messages::table)
                .values(&insertable)
                .execute(&*connection)
        })
        .await?;

        Ok(())
    }
}
#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_decline_messages"]
struct InsertableDeclineMessage {
    swap_id: Text<SwapId>,
    reason: Option<String>,
}

#[async_trait]
impl Save<Decline> for Sqlite {
    async fn save(&self, message: Decline) -> anyhow::Result<()> {
        let Decline {
            swap_id,
            reason: _reason, /* we don't map reason to a DB type because will be gone soon
                              * (hopefully) */
        } = message;

        let insertable = InsertableDeclineMessage {
            swap_id: Text(swap_id),
            reason: None,
        };

        self.do_in_transaction(|connection| {
            diesel::insert_into(rfc003_decline_messages::table)
                .values(&insertable)
                .execute(&*connection)
        })
        .await?;

        Ok(())
    }
}
