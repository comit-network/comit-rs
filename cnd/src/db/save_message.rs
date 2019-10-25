use crate::{
    db::{
        models::{ChainId, EthereumAddress, Satoshis, SqlText},
        schema::{self, *},
        Error, Sqlite,
    },
    swap_protocols::{
        ledger::{Bitcoin, Ethereum},
        rfc003::messages::{AcceptResponseBody, DeclineResponseBody, Request},
        HashFunction, SwapId,
    },
};
use diesel::RunQueryDsl;
use ethereum_support::U256;

pub trait SaveMessage<M> {
    fn save_message(&self, message: M) -> Result<(), Error>;
}

pub trait SaveRfc003Messages:
    SaveMessage<Request<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::EtherQuantity>>
    + SaveMessage<Request<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::Erc20Token>>
    + SaveMessage<Request<Ethereum, Bitcoin, ethereum_support::EtherQuantity, bitcoin::Amount>>
    + SaveMessage<Request<Ethereum, Bitcoin, ethereum_support::Erc20Token, bitcoin::Amount>>
    + SaveMessage<AcceptResponseBody<Bitcoin, Ethereum>>
    + SaveMessage<AcceptResponseBody<Ethereum, Bitcoin>>
    + SaveMessage<DeclineResponseBody>
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

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_bitcoin_ethereum_bitcoin_ether_request_messages"]
struct InsertableBitcoinEthereumBitcoinEtherRequestMessage {
    swap_id: SqlText<SwapId>,
    bitcoin_network: SqlText<bitcoin::Network>,
    ethereum_chain_id: ChainId,
    bitcoin_amount: SqlText<Satoshis>,
    ether_amount: SqlText<U256>,
    hash_function: SqlText<HashFunction>,
    bitcoin_refund_identity: SqlText<bitcoin::PublicKey>,
    ethereum_redeem_identity: SqlText<EthereumAddress>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Request<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::EtherQuantity>) -> Result<(), Error> {
        use schema::rfc003_bitcoin_ethereum_bitcoin_ether_request_messages::dsl::*;

        diesel::insert_into(rfc003_bitcoin_ethereum_bitcoin_ether_request_messages)
            .values(&InsertableBitcoinEthereumBitcoinEtherRequestMessage {
            swap_id: SqlText(message.id),
            bitcoin_network: SqlText(message.alpha_ledger.network),
            ethereum_chain_id: ChainId(message.beta_ledger.chain_id.into()),
            bitcoin_amount: SqlText(Satoshis(message.alpha_asset.as_sat())),
            ether_amount: SqlText(message.beta_asset.wei()),
            hash_function: SqlText(message.hash_function),
            bitcoin_refund_identity: SqlText(message.alpha_ledger_refund_identity.into_inner()),
            ethereum_redeem_identity: SqlText(EthereumAddress(message.beta_ledger_redeem_identity)),
        })
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
    ethereum_chain_id: ChainId,
    bitcoin_amount: SqlText<Satoshis>,
    erc20_amount: SqlText<U256>,
    erc20_token_contract: SqlText<EthereumAddress>,
    hash_function: SqlText<HashFunction>,
    bitcoin_refund_identity: SqlText<bitcoin::PublicKey>,
    ethereum_redeem_identity: SqlText<EthereumAddress>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Request<Bitcoin, Ethereum, bitcoin::Amount, ethereum_support::Erc20Token>) -> Result<(), Error> {
        use schema::rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages::dsl::*;

        diesel::insert_into(rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages)
            .values(&InsertableBitcoinEthereumBitcoinErc20RequestMessage {
            swap_id: SqlText(message.id),
            bitcoin_network: SqlText(message.alpha_ledger.network),
            ethereum_chain_id: ChainId(message.beta_ledger.chain_id.into()),
            bitcoin_amount: SqlText(Satoshis(message.alpha_asset.as_sat())),
            erc20_amount: SqlText(message.beta_asset.quantity.0),
            erc20_token_contract: SqlText(EthereumAddress(message.beta_asset.token_contract)),
            hash_function: SqlText(message.hash_function),
            bitcoin_refund_identity: SqlText(message.alpha_ledger_refund_identity.into_inner()),
            ethereum_redeem_identity: SqlText(EthereumAddress(message.beta_ledger_redeem_identity)),
        })
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_ether_bitcoin_request_messages"]
struct InsertableEthereumBitcoinEtherBitcoinRequestMessage {
    swap_id: SqlText<SwapId>,
    ethereum_chain_id: ChainId,
    bitcoin_network: SqlText<bitcoin::Network>,
    ether_amount: SqlText<U256>,
    bitcoin_amount: SqlText<Satoshis>,
    hash_function: SqlText<HashFunction>,
    ethereum_refund_identity: SqlText<EthereumAddress>,
    bitcoin_redeem_identity: SqlText<bitcoin::PublicKey>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Request<Ethereum, Bitcoin, ethereum_support::EtherQuantity, bitcoin::Amount>) -> Result<(), Error> {
        use schema::rfc003_ethereum_bitcoin_ether_bitcoin_request_messages::dsl::*;

        diesel::insert_into(rfc003_ethereum_bitcoin_ether_bitcoin_request_messages)
            .values(&InsertableEthereumBitcoinEtherBitcoinRequestMessage {
            swap_id: SqlText(message.id),
            ethereum_chain_id: ChainId(message.alpha_ledger.chain_id.into()),
            bitcoin_network: SqlText(message.beta_ledger.network),
            ether_amount: SqlText(message.alpha_asset.wei()),
            bitcoin_amount: SqlText(Satoshis(message.beta_asset.as_sat())),
            hash_function: SqlText(message.hash_function),
            ethereum_refund_identity: SqlText(EthereumAddress(message.alpha_ledger_refund_identity)),
            bitcoin_redeem_identity: SqlText(message.beta_ledger_redeem_identity.into_inner()),
        })
            .execute(&connection)
            .map(|_| ())
            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Copy, Clone)]
#[table_name = "rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages"]
struct InsertableEthereumBitcoinErc20BitcoinRequestMessage {
    swap_id: SqlText<SwapId>,
    ethereum_chain_id: ChainId,
    bitcoin_network: SqlText<bitcoin::Network>,
    erc20_amount: SqlText<U256>,
    erc20_token_contract: SqlText<EthereumAddress>,
    bitcoin_amount: SqlText<Satoshis>,
    hash_function: SqlText<HashFunction>,
    ethereum_refund_identity: SqlText<EthereumAddress>,
    bitcoin_redeem_identity: SqlText<bitcoin::PublicKey>,
}

impl_save_message! {
    fn save_message(connection: SqliteConnection, message: Request<Ethereum, Bitcoin, ethereum_support::Erc20Token, bitcoin::Amount>) -> Result<(), Error> {
        use schema::rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages::dsl::*;

        diesel::insert_into(rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages)
            .values(&InsertableEthereumBitcoinErc20BitcoinRequestMessage {
            swap_id: SqlText(message.id),
            ethereum_chain_id: ChainId(message.alpha_ledger.chain_id.into()),
            bitcoin_network: SqlText(message.beta_ledger.network),
            erc20_amount: SqlText(message.alpha_asset.quantity.0),
            erc20_token_contract: SqlText(EthereumAddress(message.alpha_asset.token_contract)),
            bitcoin_amount: SqlText(Satoshis(message.beta_asset.as_sat())),
            hash_function: SqlText(message.hash_function),
            ethereum_refund_identity: SqlText(EthereumAddress(message.alpha_ledger_refund_identity)),
            bitcoin_redeem_identity: SqlText(message.beta_ledger_redeem_identity.into_inner()),
        })
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
    fn save_message(_connection: SqliteConnection, _message: AcceptResponseBody<Bitcoin, Ethereum>) -> Result<(), Error> {
            unimplemented!("accept message doesn't contain the swap id");
//        use schema::rfc003_ethereum_bitcoin_accept_messages::dsl::*;
//
//        diesel::insert_into(rfc003_ethereum_bitcoin_accept_messages)
//            .values(&InsertableEthereumBitcoinAcceptMessage {
//            swap_id: SqlText(message.id),
//            ethereum_redeem_identity: SqlText(EthereumAddress(message.alpha_ledger_redeem_identity)),
//            bitcoin_refund_identity: SqlText(message.beta_ledger_refund_identity.into_inner()),
//        })
//            .execute(&connection)
//            .map(|_| ())
//            .map_err(Error::Diesel)
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
    fn save_message(_connection: SqliteConnection, _message: AcceptResponseBody<Ethereum, Bitcoin>) -> Result<(), Error> {
        unimplemented!("accept message doesn't contain the swap id");
//        use schema::rfc003_bitcoin_ethereum_accept_messages::dsl::*;
//
//        diesel::insert_into(rfc003_bitcoin_ethereum_accept_messages)
//            .values(&InsertableBitcoinEthereumAcceptMessage {
//            swap_id: SqlText(message.id),
//            bitcoin_redeem_identity: SqlText(message.alpha_ledger_refund_identity.into_inner()),
//            ethereum_refund_identity: SqlText(EthereumAddress(message.beta_ledger_redeem_identity)),
//        })
//            .execute(&connection)
//            .map(|_| ())
//            .map_err(Error::Diesel)
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "rfc003_decline_messages"]
struct InsertableDeclineMessage {
    swap_id: SqlText<SwapId>,
    reason: Option<String>,
}

impl_save_message! {
    fn save_message(_connection: SqliteConnection, _message: DeclineResponseBody) -> Result<(), Error> {
        unimplemented!("decline message doesn't contain the swap id");
//        use schema::rfc003_decline_messages::dsl::*;
//
//        diesel::insert_into(rfc003_decline_messages)
//            .values(&InsertableDeclineMessage {
//            swap_id: SqlText(message.id),
//            reason: None, // ups, I don't care
//        })
//            .execute(&connection)
//            .map(|_| ())
//            .map_err(Error::Diesel)
    }
}

impl SaveRfc003Messages for Sqlite {}
