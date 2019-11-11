use crate::{
    db::{
        models::{DecimalU256, EthereumAddress, Satoshis, SqlText, U32},
        schema::{self, *},
        Error, Sqlite,
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::{Accept, Decline, Request, SecretHash},
        HashFunction, SwapId,
    },
};
use diesel::RunQueryDsl;

pub trait SaveMessage<M> {
    fn save_message(&self, message: M) -> Result<(), Error>;
}

pub trait SaveRfc003Messages:
    SaveMessage<Request<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::EtherQuantity>>
    + SaveMessage<Request<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::Erc20Token>>
    + SaveMessage<Request<Ethereum, Bitcoin, ethereum_support::EtherQuantity, bitcoin::Amount>>
    + SaveMessage<Request<Ethereum, Bitcoin, ethereum_support::Erc20Token, bitcoin::Amount>>
    + SaveMessage<Accept<Bitcoin, Ethereum>>
    + SaveMessage<Accept<Ethereum, Bitcoin>>
    + SaveMessage<Decline>
{
}

macro_rules! impl_save_message {
    (
        fn save_message($conn:ident : SqliteConnection, $var:ident : $message:ty) ->
        $ret:ty
        $body:block
    ) => {
        impl SaveMessage<$message> for Sqlite {
            fn save_message(&self, $var: $message) -> $ret {
                let $conn = self.connect().unwrap();

                $body
            }
        }
    };
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_bitcoin_ethereum_bitcoin_ether_request_messages"]
struct InsertableBitcoinEthereumBitcoinEtherRequestMessage {
    swap_id: SqlText<SwapId>,
    bitcoin_network: SqlText<bitcoin::Network>,
    ethereum_chain_id: U32,
    bitcoin_amount: SqlText<Satoshis>,
    ether_amount: SqlText<DecimalU256>,
    hash_function: SqlText<HashFunction>,
    bitcoin_refund_identity: SqlText<bitcoin::PublicKey>,
    ethereum_redeem_identity: SqlText<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: SqlText<SecretHash>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Request<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::EtherQuantity>) -> Result<(), Error> {
        let Request {
            id,
            alpha_ledger,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash
        } = message;

        let insertable = InsertableBitcoinEthereumBitcoinEtherRequestMessage {
            swap_id: SqlText(id),
            bitcoin_network: SqlText(alpha_ledger.network),
            ethereum_chain_id: U32(beta_ledger.chain_id.into()),
            bitcoin_amount: SqlText(Satoshis(alpha_asset.as_sat())),
            ether_amount: SqlText(DecimalU256(beta_asset.wei())),
            hash_function: SqlText(hash_function),
            bitcoin_refund_identity: SqlText(alpha_ledger_refund_identity.into_inner()),
            ethereum_redeem_identity: SqlText(EthereumAddress(beta_ledger_redeem_identity)),
            bitcoin_expiry: U32(alpha_expiry.into()),
            ethereum_expiry: U32(beta_expiry.into()),
            secret_hash: SqlText(secret_hash)
        };

        diesel::insert_into(schema::rfc003_bitcoin_ethereum_bitcoin_ether_request_messages::dsl::rfc003_bitcoin_ethereum_bitcoin_ether_request_messages)
            .values(&insertable)
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages"]
struct InsertableBitcoinEthereumBitcoinErc20RequestMessage {
    swap_id: SqlText<SwapId>,
    bitcoin_network: SqlText<bitcoin::Network>,
    ethereum_chain_id: U32,
    bitcoin_amount: SqlText<Satoshis>,
    erc20_amount: SqlText<DecimalU256>,
    erc20_token_contract: SqlText<EthereumAddress>,
    hash_function: SqlText<HashFunction>,
    bitcoin_refund_identity: SqlText<bitcoin::PublicKey>,
    ethereum_redeem_identity: SqlText<EthereumAddress>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: SqlText<SecretHash>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Request<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::Erc20Token>) -> Result<(), Error> {
        let Request {
            id,
            alpha_ledger,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash
        } = message;

        let insertable = InsertableBitcoinEthereumBitcoinErc20RequestMessage {
            swap_id: SqlText(id),
            bitcoin_network: SqlText(alpha_ledger.network),
            ethereum_chain_id: U32(beta_ledger.chain_id.into()),
            bitcoin_amount: SqlText(Satoshis(alpha_asset.as_sat())),
            erc20_amount: SqlText(DecimalU256(beta_asset.quantity.0)),
            erc20_token_contract: SqlText(EthereumAddress(beta_asset.token_contract)),
            hash_function: SqlText(hash_function),
            bitcoin_refund_identity: SqlText(alpha_ledger_refund_identity.into_inner()),
            ethereum_redeem_identity: SqlText(EthereumAddress(beta_ledger_redeem_identity)),
            bitcoin_expiry: U32(alpha_expiry.into()),
            ethereum_expiry: U32(beta_expiry.into()),
                    secret_hash: SqlText(secret_hash)
};

        diesel::insert_into(schema::rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages::dsl::rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages)
            .values(&insertable)
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_ether_bitcoin_request_messages"]
struct InsertableEthereumBitcoinEtherBitcoinRequestMessage {
    swap_id: SqlText<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: SqlText<bitcoin::Network>,
    ether_amount: SqlText<DecimalU256>,
    bitcoin_amount: SqlText<Satoshis>,
    hash_function: SqlText<HashFunction>,
    ethereum_refund_identity: SqlText<EthereumAddress>,
    bitcoin_redeem_identity: SqlText<bitcoin::PublicKey>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: SqlText<SecretHash>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Request<Ethereum, Bitcoin, ethereum_support::EtherQuantity, bitcoin::Amount>) -> Result<(), Error> {
        let Request {
            id,
            alpha_ledger,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash
        } = message;

        let insertable = InsertableEthereumBitcoinEtherBitcoinRequestMessage {
            swap_id: SqlText(id),
            ethereum_chain_id: U32(alpha_ledger.chain_id.into()),
            bitcoin_network: SqlText(beta_ledger.network),
            ether_amount: SqlText(DecimalU256(alpha_asset.wei())),
            bitcoin_amount: SqlText(Satoshis(beta_asset.as_sat())),
            hash_function: SqlText(hash_function),
            ethereum_refund_identity: SqlText(EthereumAddress(alpha_ledger_refund_identity)),
            bitcoin_redeem_identity: SqlText(beta_ledger_redeem_identity.into_inner()),
            ethereum_expiry: U32(alpha_expiry.into()),
            bitcoin_expiry: U32(beta_expiry.into()),
                    secret_hash: SqlText(secret_hash)
};

        diesel::insert_into(schema::rfc003_ethereum_bitcoin_ether_bitcoin_request_messages::dsl::rfc003_ethereum_bitcoin_ether_bitcoin_request_messages)
            .values(&insertable)
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages"]
struct InsertableEthereumBitcoinErc20BitcoinRequestMessage {
    swap_id: SqlText<SwapId>,
    ethereum_chain_id: U32,
    bitcoin_network: SqlText<bitcoin::Network>,
    erc20_amount: SqlText<DecimalU256>,
    erc20_token_contract: SqlText<EthereumAddress>,
    bitcoin_amount: SqlText<Satoshis>,
    hash_function: SqlText<HashFunction>,
    ethereum_refund_identity: SqlText<EthereumAddress>,
    bitcoin_redeem_identity: SqlText<bitcoin::PublicKey>,
    bitcoin_expiry: U32,
    ethereum_expiry: U32,
    secret_hash: SqlText<SecretHash>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Request<Ethereum, Bitcoin, ethereum_support::Erc20Token, bitcoin::Amount>) -> Result<(), Error> {
        let Request {
            id,
            alpha_ledger,
            alpha_asset,
            beta_ledger,
            beta_asset,
            hash_function,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash
        } = message;

        let insertable = InsertableEthereumBitcoinErc20BitcoinRequestMessage {
            swap_id: SqlText(id),
            ethereum_chain_id: U32(alpha_ledger.chain_id.into()),
            bitcoin_network: SqlText(beta_ledger.network),
            erc20_amount: SqlText(DecimalU256(alpha_asset.quantity.0)),
            erc20_token_contract: SqlText(EthereumAddress(alpha_asset.token_contract)),
            bitcoin_amount: SqlText(Satoshis(beta_asset.as_sat())),
            hash_function: SqlText(hash_function),
            ethereum_refund_identity: SqlText(EthereumAddress(alpha_ledger_refund_identity)),
            bitcoin_redeem_identity: SqlText(beta_ledger_redeem_identity.into_inner()),
            ethereum_expiry: U32(alpha_expiry.into()),
            bitcoin_expiry: U32(beta_expiry.into()),
                    secret_hash: SqlText(secret_hash)
};

        diesel::insert_into(schema::rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages::dsl::rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages)
            .values(&insertable)
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_accept_messages"]
struct InsertableEthereumBitcoinAcceptMessage {
    swap_id: SqlText<SwapId>,
    ethereum_redeem_identity: SqlText<EthereumAddress>,
    bitcoin_refund_identity: SqlText<bitcoin::PublicKey>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Accept<Bitcoin, Ethereum>) -> Result<(), Error> {
        let Accept {
            id,
            alpha_ledger_redeem_identity,
            beta_ledger_refund_identity
        } = message;

        let insertable = InsertableEthereumBitcoinAcceptMessage {
            swap_id: SqlText(id),
            ethereum_redeem_identity: SqlText(EthereumAddress(beta_ledger_refund_identity)),
            bitcoin_refund_identity: SqlText(alpha_ledger_redeem_identity.into_inner()),
        };

        diesel::insert_into(schema::rfc003_ethereum_bitcoin_accept_messages::dsl::rfc003_ethereum_bitcoin_accept_messages)
            .values(&insertable)
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_bitcoin_ethereum_accept_messages"]
struct InsertableBitcoinEthereumAcceptMessage {
    swap_id: SqlText<SwapId>,
    bitcoin_redeem_identity: SqlText<bitcoin::PublicKey>,
    ethereum_refund_identity: SqlText<EthereumAddress>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Accept<Ethereum, Bitcoin>) -> Result<(), Error> {
        let Accept {
            id,
            alpha_ledger_redeem_identity,
            beta_ledger_refund_identity
        } = message;

        let insertable = InsertableBitcoinEthereumAcceptMessage {
            swap_id: SqlText(id),
            bitcoin_redeem_identity: SqlText(beta_ledger_refund_identity.into_inner()),
            ethereum_refund_identity: SqlText(EthereumAddress(alpha_ledger_redeem_identity)),
        };

        diesel::insert_into(schema::rfc003_bitcoin_ethereum_accept_messages::dsl::rfc003_bitcoin_ethereum_accept_messages)
            .values(&insertable)
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_decline_messages"]
struct InsertableDeclineMessage {
    swap_id: SqlText<SwapId>,
    reason: Option<String>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Decline) -> Result<(), Error> {
        let Decline {
            id, reason: _reason // we don't map reason to a DB type because will be gone soon (hopefully)
        } = message;

        let insertable = InsertableDeclineMessage {
            swap_id: SqlText(id),
            reason: None,
        };

        diesel::insert_into(schema::rfc003_decline_messages::dsl::rfc003_decline_messages)
            .values(&insertable)
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

impl SaveRfc003Messages for Sqlite {}
