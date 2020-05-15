use crate::{
    db::{
        schema::{address_hints, halights, herc20s, secret_hashes, swaps},
        wrapper_types::{
            custom_sql_types::{Text, U32},
            Erc20Amount, EthereumAddress, LightningNetwork, Satoshis,
        },
        Error, Sqlite,
    },
    identity, lightning,
    swap_protocols::{halight, herc20, rfc003, HashFunction, Ledger, LocalSwapId, Role},
};
use anyhow::Context;
use diesel::{prelude::*, RunQueryDsl};
use libp2p::{Multiaddr, PeerId};

#[derive(Identifiable, Queryable, PartialEq, Debug)]
#[table_name = "swaps"]
pub struct Swap {
    id: i32,
    pub local_swap_id: Text<LocalSwapId>,
    pub role: Text<Role>,
    pub counterparty_peer_id: Text<PeerId>,
}

impl From<Swap> for InsertableSwap {
    fn from(swap: Swap) -> Self {
        InsertableSwap {
            local_swap_id: swap.local_swap_id,
            role: swap.role,
            counterparty_peer_id: swap.counterparty_peer_id,
        }
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    local_swap_id: Text<LocalSwapId>,
    role: Text<Role>,
    counterparty_peer_id: Text<PeerId>,
}

impl InsertableSwap {
    pub fn new(swap_id: LocalSwapId, counterparty: PeerId, role: Role) -> Self {
        InsertableSwap {
            local_swap_id: Text(swap_id),
            role: Text(role),
            counterparty_peer_id: Text(counterparty),
        }
    }
}

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "secret_hashes"]
pub struct SecretHash {
    id: i32,
    swap_id: i32,
    pub secret_hash: Text<rfc003::SecretHash>,
}

#[derive(Insertable, Debug, Clone, Copy)]
#[table_name = "secret_hashes"]
pub struct InsertableSecretHash {
    swap_id: i32,
    secret_hash: Text<rfc003::SecretHash>,
}

#[derive(Clone, Debug, Identifiable, Queryable, PartialEq)]
#[table_name = "address_hints"]
pub struct AddressHint {
    id: i32,
    pub peer_id: Text<PeerId>,
    pub address_hint: Text<Multiaddr>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "address_hints"]
pub struct InsertableAddressHint {
    peer_id: Text<PeerId>,
    address_hint: Text<Multiaddr>,
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "herc20s"]
pub struct Herc20 {
    id: i32,
    swap_id: i32,
    pub amount: Text<Erc20Amount>,
    pub chain_id: U32,
    pub expiry: U32,
    pub hash_function: Text<HashFunction>,
    pub token_contract: Text<EthereumAddress>,
    pub redeem_identity: Option<Text<EthereumAddress>>,
    pub refund_identity: Option<Text<EthereumAddress>>,
    pub ledger: Text<Ledger>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "herc20s"]
pub struct InsertableHerc20 {
    pub swap_id: i32,
    pub amount: Text<Erc20Amount>,
    pub chain_id: U32,
    pub expiry: U32,
    pub hash_function: Text<HashFunction>,
    pub token_contract: Text<EthereumAddress>,
    pub redeem_identity: Option<Text<EthereumAddress>>,
    pub refund_identity: Option<Text<EthereumAddress>>,
    pub ledger: Text<Ledger>,
}

pub trait IntoInsertable {
    type Insertable;

    fn into_insertable(self, swap_id: i32, role: Role, ledger: Ledger) -> Self::Insertable;
}

pub trait Insert<I> {
    fn insert(&self, connection: &SqliteConnection, insertable: &I) -> anyhow::Result<()>;
}

impl IntoInsertable for herc20::CreatedSwap {
    type Insertable = InsertableHerc20;

    fn into_insertable(self, swap_id: i32, role: Role, ledger: Ledger) -> Self::Insertable {
        let redeem_identity = match role {
            Role::Alice => None,
            Role::Bob => Some(Text(EthereumAddress::from(self.identity))),
        };
        let refund_identity = match role {
            Role::Alice => Some(Text(EthereumAddress::from(self.identity))),
            Role::Bob => None,
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        InsertableHerc20 {
            swap_id,
            amount: Text(self.amount.into()),
            chain_id: U32(self.chain_id),
            expiry: U32(self.absolute_expiry),
            hash_function: Text(HashFunction::Sha256),
            token_contract: Text(self.token_contract.into()),
            redeem_identity,
            refund_identity,
            ledger: Text(ledger),
        }
    }
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "halights"]
pub struct Halight {
    id: i32,
    swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<LightningNetwork>,
    pub chain: String,
    pub cltv_expiry: U32,
    pub hash_function: Text<HashFunction>,
    pub redeem_identity: Option<Text<lightning::PublicKey>>,
    pub refund_identity: Option<Text<lightning::PublicKey>>,
    pub ledger: Text<Ledger>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "halights"]
pub struct InsertableHalight {
    pub swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<LightningNetwork>,
    pub chain: String,
    pub cltv_expiry: U32,
    pub hash_function: Text<HashFunction>,
    pub redeem_identity: Option<Text<lightning::PublicKey>>,
    pub refund_identity: Option<Text<lightning::PublicKey>>,
    pub ledger: Text<Ledger>,
}

impl IntoInsertable for halight::CreatedSwap {
    type Insertable = InsertableHalight;

    fn into_insertable(self, swap_id: i32, role: Role, ledger: Ledger) -> Self::Insertable {
        let redeem_identity = match role {
            Role::Alice => Some(Text(self.identity)),
            Role::Bob => None,
        };
        let refund_identity = match role {
            Role::Alice => None,
            Role::Bob => Some(Text(self.identity)),
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        InsertableHalight {
            swap_id,
            amount: Text(self.amount.into()),
            network: Text(self.network.into()),
            chain: "bitcoin".to_string(), // We currently only support Lightning on top of Bitcoin.
            cltv_expiry: U32(self.cltv_expiry),
            hash_function: Text(HashFunction::Sha256),
            redeem_identity,
            refund_identity,
            ledger: Text(ledger),
        }
    }
}

impl Insert<InsertableHerc20> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHerc20,
    ) -> anyhow::Result<()> {
        diesel::insert_into(herc20s::dsl::herc20s)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}

impl Insert<InsertableHalight> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHalight,
    ) -> anyhow::Result<()> {
        diesel::insert_into(halights::dsl::halights)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}

macro_rules! swap_id_fk {
    ($local_swap_id:expr) => {
        swaps::table
            .filter(swaps::local_swap_id.eq(Text($local_swap_id)))
            .select(swaps::id)
    };
}

impl Sqlite {
    pub async fn role(&self, swap_id: LocalSwapId) -> anyhow::Result<Role> {
        let swap = self.load_swap(swap_id).await?;
        Ok(swap.role.0)
    }

    pub fn save_swap(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableSwap,
    ) -> anyhow::Result<i32> {
        diesel::insert_into(swaps::dsl::swaps)
            .values(insertable)
            .execute(connection)?;

        let swap_id = swap_id_fk!(insertable.local_swap_id.0).first(connection)?;

        Ok(swap_id)
    }

    pub async fn load_swap(&self, swap_id: LocalSwapId) -> anyhow::Result<Swap> {
        let record: Swap = self
            .do_in_transaction(|connection| {
                let key = Text(swap_id);

                swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(connection)
                    .optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok(record)
    }

    pub fn insert_secret_hash(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        secret_hash: rfc003::SecretHash,
    ) -> anyhow::Result<()> {
        let swap_id = swap_id_fk!(local_swap_id)
            .first(connection)
            .with_context(|| {
                format!(
                    "failed to find swap_id foreign key for swap {}",
                    local_swap_id
                )
            })?;
        let insertable = InsertableSecretHash {
            swap_id,
            secret_hash: Text(secret_hash),
        };

        diesel::insert_into(secret_hashes::table)
            .values(insertable)
            .execute(&*connection)
            .with_context(|| format!("failed to insert secret hash for swap {}", local_swap_id))?;

        Ok(())
    }

    pub async fn load_secret_hash(
        &self,
        swap_id: LocalSwapId,
    ) -> anyhow::Result<rfc003::SecretHash> {
        let record: SecretHash = self
            .do_in_transaction(|connection| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(connection)?;

                SecretHash::belonging_to(&swap).first(connection).optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok(record.secret_hash.0)
    }

    pub fn update_halight_refund_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Lightning,
    ) -> anyhow::Result<()> {
        diesel::update(halights::table)
            .filter(halights::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(halights::refund_identity.eq(Text(identity)))
            .execute(connection)
            .with_context(|| {
                format!(
                    "failed to update halight refund identity for swap {}",
                    local_swap_id
                )
            })?;

        Ok(())
    }

    pub fn update_halight_redeem_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Lightning,
    ) -> anyhow::Result<()> {
        diesel::update(halights::table)
            .filter(halights::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(halights::redeem_identity.eq(Text(identity)))
            .execute(connection)
            .with_context(|| {
                format!(
                    "failed to update halight redeem identity for swap {}",
                    local_swap_id
                )
            })?;

        Ok(())
    }

    pub fn update_herc20_refund_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        diesel::update(herc20s::table)
            .filter(herc20s::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(herc20s::refund_identity.eq(Text(identity)))
            .execute(connection)
            .with_context(|| {
                format!(
                    "failed to update herc20 refund identity for swap {}",
                    local_swap_id
                )
            })?;

        Ok(())
    }

    pub fn update_herc20_redeem_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        diesel::update(herc20s::table)
            .filter(herc20s::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(herc20s::redeem_identity.eq(Text(identity)))
            .execute(connection)
            .with_context(|| {
                format!(
                    "failed to update herc20 redeem identity for swap {}",
                    local_swap_id
                )
            })?;

        Ok(())
    }

    pub fn save_address_hint(
        &self,
        connection: &SqliteConnection,
        peer_id: PeerId,
        address_hint: libp2p::Multiaddr,
    ) -> anyhow::Result<()> {
        let insertable = InsertableAddressHint {
            peer_id: Text(peer_id),
            address_hint: Text(address_hint),
        };

        diesel::insert_into(address_hints::dsl::address_hints)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }

    pub async fn load_address_hint(&self, peer_id: &PeerId) -> anyhow::Result<libp2p::Multiaddr> {
        let record: AddressHint = self
            .do_in_transaction(|connection| {
                let key = Text(peer_id);

                address_hints::table
                    .filter(address_hints::peer_id.eq(key))
                    .first(connection)
                    .optional()
            })
            .await?
            .ok_or(Error::PeerIdNotFound)?;

        Ok(record.address_hint.0)
    }

    pub async fn load_herc20(&self, swap_id: LocalSwapId) -> anyhow::Result<Herc20> {
        let record: Herc20 = self
            .do_in_transaction(|connection| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(connection)?;

                Herc20::belonging_to(&swap).first(connection).optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok(record)
    }

    pub async fn load_halight(&self, swap_id: LocalSwapId) -> anyhow::Result<Halight> {
        let record: Halight = self
            .do_in_transaction(|connection| {
                let key = Text(swap_id);

                let swap: Swap = swaps::table
                    .filter(swaps::local_swap_id.eq(key))
                    .first(connection)?;

                Halight::belonging_to(&swap).first(connection).optional()
            })
            .await?
            .ok_or(Error::SwapNotFound)?;

        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lightning;
    use std::{path::PathBuf, str::FromStr};

    fn temp_db() -> PathBuf {
        let temp_file = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .unwrap();

        temp_file.into_temp_path().to_path_buf()
    }

    fn insertable_swap() -> InsertableSwap {
        let swap_id =
            LocalSwapId::from_str("ad2652ca-ecf2-4cc6-b35c-b4351ac28a34").expect("valid swap id");
        let role = Role::Alice;
        let peer_id = PeerId::from_str("QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY")
            .expect("valid peer id");

        InsertableSwap {
            local_swap_id: Text(swap_id),
            role: Text(role),
            counterparty_peer_id: Text(peer_id),
        }
    }

    impl PartialEq<InsertableSwap> for Swap {
        fn eq(&self, other: &InsertableSwap) -> bool {
            self.local_swap_id == other.local_swap_id
                && self.role == other.role
                && self.counterparty_peer_id == other.counterparty_peer_id
        }
    }

    impl PartialEq<InsertableHerc20> for Herc20 {
        fn eq(&self, other: &InsertableHerc20) -> bool {
            self.amount == other.amount
                && self.chain_id == other.chain_id
                && self.expiry == other.expiry
                && self.hash_function == other.hash_function
                && self.token_contract == other.token_contract
                && self.redeem_identity == other.redeem_identity
                && self.refund_identity == other.refund_identity
                && self.ledger == other.ledger
        }
    }

    impl PartialEq<InsertableHalight> for Halight {
        fn eq(&self, other: &InsertableHalight) -> bool {
            self.amount == other.amount
                && self.network == other.network
                && self.chain == other.chain
                && self.cltv_expiry == other.cltv_expiry
                && self.hash_function == other.hash_function
                && self.redeem_identity == other.redeem_identity
                && self.refund_identity == other.refund_identity
                && self.ledger == other.ledger
        }
    }

    #[tokio::test]
    async fn roundtrip_swap() {
        let path = temp_db();
        let db = Sqlite::new(&path).expect("a new db");

        let given = insertable_swap();
        let swap_id = given.local_swap_id.0;

        db.do_in_transaction(|conn| db.save_swap(conn, &given))
            .await
            .expect("to be able to save a swap");

        let loaded = db
            .load_swap(swap_id)
            .await
            .expect("to be able to load a previously saved swap");

        assert_eq!(loaded, given)
    }

    #[tokio::test]
    async fn roundtrip_secret_hash() {
        let path = temp_db();
        let db = Sqlite::new(&path).expect("a new db");

        let swap = insertable_swap();
        let swap_id = swap.local_swap_id.0;

        db.do_in_transaction(|conn| db.save_swap(conn, &swap))
            .await
            .expect("to be able to save a swap");

        let secret_hash = rfc003::SecretHash::from_str(
            "bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf\
             bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf",
        )
        .expect("valid secret hash");

        db.do_in_transaction(|conn| db.insert_secret_hash(conn, swap_id, secret_hash))
            .await
            .expect("to be able to save secret hash");

        let loaded = db
            .load_secret_hash(swap_id)
            .await
            .expect("to be able to load a previously saved secret hash");

        assert_eq!(loaded, secret_hash)
    }

    #[tokio::test]
    async fn roundtrip_address_hint() {
        let path = temp_db();
        let db = Sqlite::new(&path).expect("a new db");

        let swap = insertable_swap();

        db.do_in_transaction(|conn| db.save_swap(conn, &swap))
            .await
            .expect("to be able to save a swap");

        let peer_id = PeerId::from_str("QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY")
            .expect("valid peer id");
        let multi_addr = "/ip4/80.123.90.4/tcp/5432";
        let address_hint: Multiaddr = multi_addr.parse().expect("valid multiaddress");

        db.do_in_transaction(|conn| {
            db.save_address_hint(conn, peer_id.clone(), address_hint.clone())
        })
        .await
        .expect("to be able to save address hint");

        let loaded = db
            .load_address_hint(&peer_id)
            .await
            .expect("to be able to load a previously saved address hint");

        assert_eq!(loaded, address_hint)
    }

    #[tokio::test]
    async fn roundtrip_herc20s() {
        let path = temp_db();
        let db = Sqlite::new(&path).expect("a new db");

        let swap = insertable_swap();
        let local_swap_id = swap.local_swap_id.0;

        let swap_id = db
            .do_in_transaction(|conn| db.save_swap(conn, &swap))
            .await
            .expect("to be able to save a swap");

        let amount = Erc20Amount::from_str("12345").expect("valid ERC20 amount");
        let ethereum_identity =
            EthereumAddress::from_str("1111e8be41b21f651a71aaB1A85c6813b8bBcCf8")
                .expect("valid etherum identity");
        let redeem_identity = EthereumAddress::from_str("2222e8be41b21f651a71aaB1A85c6813b8bBcCf8")
            .expect("valid redeem identity");
        let refund_identity = EthereumAddress::from_str("3333e8be41b21f651a71aaB1A85c6813b8bBcCf8")
            .expect("valid refund identity");

        let given = InsertableHerc20 {
            swap_id,
            amount: Text(amount),
            chain_id: U32(1337),
            expiry: U32(123),
            hash_function: Text(HashFunction::Sha256),
            token_contract: Text(ethereum_identity),
            redeem_identity: Some(Text(redeem_identity)),
            refund_identity: Some(Text(refund_identity)),
            ledger: Text(Ledger::Alpha),
        };

        db.do_in_transaction(|conn| db.insert(conn, &given))
            .await
            .expect("to be able to save swap details");

        let loaded = db
            .load_herc20(local_swap_id)
            .await
            .expect("to be able to load a previously saved swap details");

        assert_eq!(loaded, given)
    }

    #[tokio::test]
    async fn roundtrip_halights() {
        let path = temp_db();
        let db = Sqlite::new(&path).expect("a new db");

        let swap = insertable_swap();
        let local_swap_id = swap.local_swap_id.0;

        let swap_id = db
            .do_in_transaction(|conn| db.save_swap(conn, &swap))
            .await
            .expect("to be able to save a swap");

        let amount = Satoshis::from_str("12345").expect("valid ERC20 amount");

        let redeem_identity = lightning::PublicKey::random();
        let refund_identity = lightning::PublicKey::random();

        let given = InsertableHalight {
            swap_id,
            amount: Text(amount),
            network: Text(LightningNetwork::Testnet),
            chain: "bitcoin".to_string(),
            cltv_expiry: U32(456),
            hash_function: Text(HashFunction::Sha256),
            redeem_identity: Some(Text(redeem_identity)),
            refund_identity: Some(Text(refund_identity)),
            ledger: Text(Ledger::Alpha),
        };

        db.do_in_transaction(|conn| db.insert(conn, &given))
            .await
            .expect("to be able to save swap details");

        let loaded = db
            .load_halight(local_swap_id)
            .await
            .expect("to be able to load a previously saved swap details");

        assert_eq!(loaded, given)
    }
}
