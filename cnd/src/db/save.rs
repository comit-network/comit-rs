use crate::{
    asset,
    db::{
        schema::{self, *},
        wrapper_types::{
            custom_sql_types::{Text, U32},
            BitcoinNetwork, Erc20Amount, Ether, EthereumAddress, Satoshis,
        },
        CreatedSwap, FinalizedSwap, Sqlite, Swap,
    },
    identity,
    swap_protocols::{
        halight, han,
        ledger::{self, Ethereum},
        rfc003::{Accept, Decline, Request, SecretHash, SwapId},
        HashFunction, Role,
    },
};
use async_trait::async_trait;
use diesel::RunQueryDsl;
use impl_template::impl_template;
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
}

#[impl_template]
#[async_trait]
impl
    Save<
        Request<
            ((
                ledger::bitcoin::Mainnet,
                ledger::bitcoin::Testnet,
                ledger::bitcoin::Regtest,
            )),
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > for Sqlite
{
    async fn save(
        &self,
        message: Request<
            __TYPE0__,
            Ethereum,
            asset::Bitcoin,
            asset::Ether,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<()> {
        let Request {
            swap_id,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
            ..
        } = message;

        let insertable = InsertableBitcoinEthereumBitcoinEtherRequestMessage {
            swap_id: Text(swap_id),
            bitcoin_network: Text(BitcoinNetwork::from(__TYPE0__)),
            ethereum_chain_id: U32(beta_ledger.chain_id.into()),
            bitcoin_amount: Text(alpha_asset.into()),
            ether_amount: Text(beta_asset.into()),
            hash_function: Text(hash_function),
            bitcoin_refund_identity: Text(alpha_ledger_refund_identity.into()),
            ethereum_redeem_identity: Text(beta_ledger_redeem_identity.into()),
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

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages"]
struct InsertableBitcoinEthereumBitcoinErc20RequestMessage {
    swap_id: Text<SwapId>,
    bitcoin_network: Text<BitcoinNetwork>,
    ethereum_chain_id: U32,
    bitcoin_amount: Text<Satoshis>,
    erc20_amount: Text<Erc20Amount>,
    erc20_token_contract: Text<EthereumAddress>,
    hash_function: Text<HashFunction>,
    bitcoin_refund_identity: Text<::bitcoin::PublicKey>,
    ethereum_redeem_identity: Text<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: Text<SecretHash>,
}

#[impl_template]
#[async_trait]
impl
    Save<
        Request<
            ((
                ledger::bitcoin::Mainnet,
                ledger::bitcoin::Testnet,
                ledger::bitcoin::Regtest,
            )),
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    > for Sqlite
{
    async fn save(
        &self,
        message: Request<
            __TYPE0__,
            Ethereum,
            asset::Bitcoin,
            asset::Erc20,
            identity::Bitcoin,
            identity::Ethereum,
        >,
    ) -> anyhow::Result<()> {
        let Request {
            swap_id,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
            ..
        } = message;

        let insertable = InsertableBitcoinEthereumBitcoinErc20RequestMessage {
            swap_id: Text(swap_id),
            bitcoin_network: Text(BitcoinNetwork::from(__TYPE0__)),
            ethereum_chain_id: U32(beta_ledger.chain_id.into()),
            bitcoin_amount: Text(alpha_asset.into()),
            erc20_amount: Text(beta_asset.quantity.into()),
            erc20_token_contract: Text(beta_asset.token_contract.into()),
            hash_function: Text(hash_function),
            bitcoin_refund_identity: Text(alpha_ledger_refund_identity.into()),
            ethereum_redeem_identity: Text(beta_ledger_redeem_identity.into()),
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

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_ether_bitcoin_request_messages"]
struct InsertableEthereumBitcoinEtherBitcoinRequestMessage {
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
}

#[impl_template]
#[async_trait]
impl
    Save<
        Request<
            Ethereum,
            ((
                ledger::bitcoin::Mainnet,
                ledger::bitcoin::Testnet,
                ledger::bitcoin::Regtest,
            )),
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > for Sqlite
{
    async fn save(
        &self,
        message: Request<
            Ethereum,
            __TYPE0__,
            asset::Ether,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<()> {
        let Request {
            swap_id,
            alpha_ledger,
            alpha_asset,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
            ..
        } = message;

        let insertable = InsertableEthereumBitcoinEtherBitcoinRequestMessage {
            swap_id: Text(swap_id),
            bitcoin_network: Text(BitcoinNetwork::from(__TYPE0__)),
            ethereum_chain_id: U32(alpha_ledger.chain_id.into()),
            ether_amount: Text(alpha_asset.into()),
            bitcoin_amount: Text(beta_asset.into()),
            hash_function: Text(hash_function),
            ethereum_refund_identity: Text(alpha_ledger_refund_identity.into()),
            bitcoin_redeem_identity: Text(beta_ledger_redeem_identity.into()),
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

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages"]
struct InsertableEthereumBitcoinErc20BitcoinRequestMessage {
    swap_id: Text<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: Text<BitcoinNetwork>,
    erc20_amount: Text<Erc20Amount>,
    erc20_token_contract: Text<EthereumAddress>,
    bitcoin_amount: Text<Satoshis>,
    hash_function: Text<HashFunction>,
    ethereum_refund_identity: Text<EthereumAddress>,
    bitcoin_redeem_identity: Text<::bitcoin::PublicKey>,
    ethereum_expiry: U32,
    bitcoin_expiry: U32,
    secret_hash: Text<SecretHash>,
}

#[impl_template]
#[async_trait]
impl
    Save<
        Request<
            Ethereum,
            ((
                ledger::bitcoin::Mainnet,
                ledger::bitcoin::Testnet,
                ledger::bitcoin::Regtest,
            )),
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    > for Sqlite
{
    async fn save(
        &self,
        message: Request<
            Ethereum,
            __TYPE0__,
            asset::Erc20,
            asset::Bitcoin,
            identity::Ethereum,
            identity::Bitcoin,
        >,
    ) -> anyhow::Result<()> {
        let Request {
            swap_id,
            alpha_ledger,
            alpha_asset,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
            ..
        } = message;

        let insertable = InsertableEthereumBitcoinErc20BitcoinRequestMessage {
            swap_id: Text(swap_id),
            ethereum_chain_id: U32(alpha_ledger.chain_id.into()),
            bitcoin_network: Text(BitcoinNetwork::from(__TYPE0__)),
            erc20_amount: Text(alpha_asset.quantity.into()),
            erc20_token_contract: Text(alpha_asset.token_contract.into()),
            bitcoin_amount: Text(beta_asset.into()),
            hash_function: Text(hash_function),
            ethereum_refund_identity: Text(alpha_ledger_refund_identity.into()),
            bitcoin_redeem_identity: Text(beta_ledger_redeem_identity.into()),
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
impl Save<Accept<identity::Ethereum, identity::Bitcoin>> for Sqlite {
    async fn save(
        &self,
        message: Accept<identity::Ethereum, identity::Bitcoin>,
    ) -> anyhow::Result<()> {
        let Accept {
            swap_id,
            alpha_ledger_redeem_identity,
            beta_ledger_refund_identity,
        } = message;

        let insertable = InsertableEthereumBitcoinAcceptMessage {
            swap_id: Text(swap_id),
            ethereum_redeem_identity: Text(alpha_ledger_redeem_identity.into()),
            bitcoin_refund_identity: Text(beta_ledger_refund_identity.into()),
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
impl Save<Accept<identity::Bitcoin, identity::Ethereum>> for Sqlite {
    async fn save(
        &self,
        message: Accept<identity::Bitcoin, identity::Ethereum>,
    ) -> anyhow::Result<()> {
        let Accept {
            swap_id,
            alpha_ledger_redeem_identity,
            beta_ledger_refund_identity,
        } = message;

        let insertable = InsertableBitcoinEthereumAcceptMessage {
            swap_id: Text(swap_id),
            bitcoin_redeem_identity: Text(alpha_ledger_redeem_identity.into()),
            ethereum_refund_identity: Text(beta_ledger_refund_identity.into()),
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

#[async_trait]
impl Save<CreatedSwap<han::CreatedSwap, halight::CreatedSwap>> for Sqlite {
    async fn save(
        &self,
        _: CreatedSwap<han::CreatedSwap, halight::CreatedSwap>,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl Save<FinalizedSwap> for Sqlite {
    async fn save(&self, _: FinalizedSwap) -> anyhow::Result<()> {
        unimplemented!()
    }
}
