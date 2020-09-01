use crate::{
    asset, bitcoin, halbit, hbit, herc20, identity, lightning,
    storage::{
        db::{
            schema::{
                btc_dai_orders, halbits, hbits, herc20s, order_hbit_params, order_herc20_params,
                order_swaps, orders, secret_hashes, swap_contexts, swaps,
            },
            wrapper_types::{
                custom_sql_types::{Text, U32},
                BitcoinNetwork, Erc20Amount, Satoshis,
            },
            Sqlite,
        },
        NoOrderExists, NotOpen,
    },
    LocalSwapId, LockProtocol, Role, Side,
};
use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use comit::{asset::Erc20Quantity, ethereum, ethereum::ChainId, OrderId, Position};
use diesel::{prelude::*, RunQueryDsl};
use libp2p::PeerId;
use std::ops::Add;
use time::OffsetDateTime;

macro_rules! swap_id_fk {
    ($local_swap_id:expr) => {
        swaps::table
            .filter(swaps::local_swap_id.eq(Text($local_swap_id)))
            .select(swaps::id)
    };
}

#[derive(Identifiable, Queryable, PartialEq, Debug)]
#[table_name = "swaps"]
pub struct Swap {
    id: i32,
    pub local_swap_id: Text<LocalSwapId>,
    pub role: Text<Role>,
    pub counterparty_peer_id: Text<PeerId>,
    pub start_of_swap: NaiveDateTime,
}

impl From<Swap> for InsertableSwap {
    fn from(swap: Swap) -> Self {
        InsertableSwap {
            local_swap_id: swap.local_swap_id,
            role: swap.role,
            counterparty_peer_id: swap.counterparty_peer_id,
            start_of_swap: swap.start_of_swap,
        }
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    local_swap_id: Text<LocalSwapId>,
    role: Text<Role>,
    counterparty_peer_id: Text<PeerId>,
    start_of_swap: NaiveDateTime,
}

impl InsertableSwap {
    pub fn insert(self, conn: &SqliteConnection) -> Result<i32> {
        let local_swap_id = self.local_swap_id.0;

        diesel::insert_into(swaps::dsl::swaps)
            .values(self)
            .execute(conn)?;

        let swap_fk = swap_id_fk!(local_swap_id).first(conn)?;

        Ok(swap_fk)
    }
}

impl InsertableSwap {
    pub fn new(
        swap_id: LocalSwapId,
        counterparty: PeerId,
        role: Role,
        start_of_swap: NaiveDateTime,
    ) -> Self {
        InsertableSwap {
            local_swap_id: Text(swap_id),
            role: Text(role),
            counterparty_peer_id: Text(counterparty),
            start_of_swap,
        }
    }
}

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[table_name = "swap_contexts"]
pub struct SwapContext {
    id: i32,
    pub local_swap_id: Text<LocalSwapId>,
    pub role: Text<Role>,
    pub alpha: Text<LockProtocol>,
    pub beta: Text<LockProtocol>,
}

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "secret_hashes"]
pub struct SecretHash {
    id: i32,
    swap_id: i32,
    pub secret_hash: Text<comit::SecretHash>,
}

#[derive(Insertable, Debug, Clone, Copy)]
#[table_name = "secret_hashes"]
pub struct InsertableSecretHash {
    swap_id: i32,
    secret_hash: Text<comit::SecretHash>,
}

impl InsertableSecretHash {
    pub fn new(swap_fk: i32, secret_hash: comit::SecretHash) -> Self {
        Self {
            swap_id: swap_fk,
            secret_hash: Text(secret_hash),
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(secret_hashes::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
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
    pub token_contract: Text<identity::Ethereum>,
    pub redeem_identity: Option<Text<identity::Ethereum>>,
    pub refund_identity: Option<Text<identity::Ethereum>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "herc20s"]
pub struct InsertableHerc20 {
    pub swap_id: i32,
    pub amount: Text<Erc20Amount>,
    pub chain_id: U32,
    pub expiry: U32,
    pub token_contract: Text<identity::Ethereum>,
    pub redeem_identity: Option<Text<identity::Ethereum>>,
    pub refund_identity: Option<Text<identity::Ethereum>>,
    pub side: Text<Side>,
}

impl InsertableHerc20 {
    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(herc20s::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

impl InsertableHerc20 {
    pub fn new(
        swap_fk: i32,
        asset: asset::Erc20,
        chain_id: ChainId,
        expiry: u32,
        redeem_identity: identity::Ethereum,
        refund_identity: identity::Ethereum,
        side: Side,
    ) -> Self {
        Self {
            swap_id: swap_fk,
            amount: Text(asset.quantity.into()),
            chain_id: u32::from(chain_id).into(),
            expiry: U32::from(expiry),
            token_contract: Text(asset.token_contract),
            redeem_identity: Some(Text(redeem_identity)),
            refund_identity: Some(Text(refund_identity)),
            side: Text(side),
        }
    }
}

impl From<Herc20> for asset::Erc20 {
    fn from(herc20: Herc20) -> asset::Erc20 {
        asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0,
        }
    }
}

pub trait IntoInsertable {
    type Insertable;

    fn into_insertable(self, swap_id: i32, role: Role, side: Side) -> Self::Insertable;
}

pub trait Insert<I> {
    fn insert(&self, connection: &SqliteConnection, insertable: &I) -> anyhow::Result<()>;
}

impl IntoInsertable for herc20::CreatedSwap {
    type Insertable = InsertableHerc20;

    fn into_insertable(self, swap_id: i32, role: Role, side: Side) -> Self::Insertable {
        let redeem_identity = match (role, side) {
            (Role::Alice, Side::Beta) | (Role::Bob, Side::Alpha) => Some(Text(self.identity)),
            _ => None,
        };
        let refund_identity = match (role, side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => Some(Text(self.identity)),
            _ => None,
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        InsertableHerc20 {
            swap_id,
            amount: Text(self.asset.quantity.into()),
            chain_id: U32(self.chain_id.into()),
            expiry: U32(self.absolute_expiry),
            token_contract: Text(self.asset.token_contract),
            redeem_identity,
            refund_identity,
            side: Text(side),
        }
    }
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "halbits"]
pub struct Halbit {
    id: i32,
    swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<BitcoinNetwork>,
    pub chain: String,
    pub cltv_expiry: U32,
    pub redeem_identity: Option<Text<lightning::PublicKey>>,
    pub refund_identity: Option<Text<lightning::PublicKey>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "halbits"]
pub struct InsertableHalbit {
    pub swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<BitcoinNetwork>,
    pub chain: String,
    pub cltv_expiry: U32,
    pub redeem_identity: Option<Text<lightning::PublicKey>>,
    pub refund_identity: Option<Text<lightning::PublicKey>>,
    pub side: Text<Side>,
}

impl From<Halbit> for asset::Bitcoin {
    fn from(halbit: Halbit) -> Self {
        halbit.amount.0.into()
    }
}

impl IntoInsertable for halbit::CreatedSwap {
    type Insertable = InsertableHalbit;

    fn into_insertable(self, swap_id: i32, role: Role, side: Side) -> Self::Insertable {
        let redeem_identity = match (role, side) {
            (Role::Alice, Side::Beta) | (Role::Bob, Side::Alpha) => Some(Text(self.identity)),
            _ => None,
        };
        let refund_identity = match (role, side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => Some(Text(self.identity)),
            _ => None,
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        InsertableHalbit {
            swap_id,
            amount: Text(self.asset.into()),
            network: Text(self.network.into()),
            chain: "bitcoin".to_string(), // We currently only support Lightning on top of Bitcoin.
            cltv_expiry: U32(self.cltv_expiry),
            redeem_identity,
            refund_identity,
            side: Text(side),
        }
    }
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "hbits"]
pub struct Hbit {
    id: i32,
    swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<BitcoinNetwork>,
    pub expiry: U32,
    pub final_identity: Text<bitcoin::Address>,
    pub transient_identity: Option<Text<bitcoin::PublicKey>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Clone, Debug)]
#[table_name = "hbits"]
pub struct InsertableHbit {
    pub swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<BitcoinNetwork>,
    pub expiry: U32,
    // TODO: Rename to make it obvious that this is OUR final address
    pub final_identity: Text<bitcoin::Address>,
    // TODO: Rename to make it obvious that this is the other party's transient identity
    pub transient_identity: Option<Text<bitcoin::PublicKey>>,
    pub side: Text<Side>,
}

impl InsertableHbit {
    pub fn new(
        swap_fk: i32,
        asset: asset::Bitcoin,
        network: bitcoin::Network,
        expiry: u32,
        final_identity: bitcoin::Address,
        transient_identity: bitcoin::PublicKey,
        side: Side,
    ) -> Self {
        Self {
            swap_id: swap_fk,
            amount: Text(asset.into()),
            network: Text(network.into()),
            expiry: expiry.into(),
            final_identity: Text(final_identity),
            transient_identity: Some(Text(transient_identity)),
            side: Text(side),
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(hbits::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[table_name = "orders"]
pub struct Order {
    pub id: i32,
    pub order_id: Text<OrderId>,
    pub position: Text<Position>,
    pub created_at: i64,
    pub open: i32,
    pub closed: i32,
    pub settling: i32,
    pub failed: i32,
    pub cancelled: i32,
}

impl Order {
    pub fn by_order_id(conn: &SqliteConnection, order_id: OrderId) -> Result<Self> {
        let order = orders::table
            .filter(orders::order_id.eq(Text(order_id)))
            .first::<Order>(conn)
            .with_context(|| NoOrderExists(order_id))?;

        Ok(order)
    }

    /// Marks the status of the current order as settling.
    ///
    /// Whilst we don't have partial order matching, this simply means updating
    /// the percent of:
    ///
    /// - `open` to `0`
    /// - `settling` to `100`
    ///
    /// Once we implement partial order matching, this will need to get more
    /// sophisticated.
    pub fn mark_as_settling(conn: &SqliteConnection, order: &Order) -> Result<()> {
        let affected_rows = diesel::update(order)
            .set((orders::settling.eq(100), orders::open.eq(0)))
            .execute(conn)?;

        if affected_rows == 0 {
            anyhow::bail!("failed to mark order {} as settling", order.order_id.0)
        }

        Ok(())
    }

    pub fn cancel(&self, conn: &SqliteConnection) -> Result<()> {
        if self.open == 0 {
            anyhow::bail!(NotOpen(self.order_id.0))
        }

        let affected_rows = diesel::update(self)
            .set((orders::cancelled.eq(self.open), orders::open.eq(0)))
            .execute(conn)?;

        if affected_rows == 0 {
            anyhow::bail!("failed to mark order {} as cancelled", self.order_id.0)
        }

        Ok(())
    }
}

pub fn all_open_btc_dai_orders(conn: &SqliteConnection) -> Result<Vec<(Order, BtcDaiOrder)>> {
    let orders = orders::table
        .inner_join(btc_dai_orders::table)
        .filter(orders::open.add(orders::settling).gt(0))
        .load::<(Order, BtcDaiOrder)>(conn)?;

    Ok(orders)
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "orders"]
pub struct InsertableOrder {
    pub order_id: Text<OrderId>,
    pub position: Text<Position>,
    pub created_at: i64,
    // TODO: Make a custom SQL type for this
    pub open: i32,
    pub closed: i32,
    pub settling: i32,
    pub failed: i32,
    pub cancelled: i32,
}

impl InsertableOrder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        order_id: OrderId,
        position: Position,
        created_at: OffsetDateTime,
        open: i32,
        closed: i32,
        settling: i32,
        failed: i32,
        cancelled: i32,
    ) -> Self {
        Self {
            order_id: Text(order_id),
            position: Text(position),
            created_at: created_at.timestamp(),
            open,
            closed,
            settling,
            failed,
            cancelled,
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<i32> {
        let order_id = self.order_id.0;

        diesel::insert_into(orders::table)
            .values(self)
            .execute(conn)?;

        let order_pk = orders::table
            .filter(orders::order_id.eq(Text(order_id)))
            .select(orders::id)
            .first(conn)?;

        Ok(order_pk)
    }
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Order)]
#[table_name = "btc_dai_orders"]
pub struct BtcDaiOrder {
    id: i32,
    pub order_id: i32,
    pub quantity: Text<Satoshis>,
    pub price: Text<Erc20Amount>,
}

impl BtcDaiOrder {
    pub fn by_order(conn: &SqliteConnection, order: &Order) -> Result<Self> {
        let params = Self::belonging_to(order)
            .first::<Self>(conn)
            .with_context(|| format!("order {} is not a BTC/DAI order", order.order_id.0))?;

        Ok(params)
    }
}

#[derive(Insertable, Clone, Debug)]
#[table_name = "btc_dai_orders"]
pub struct InsertableBtcDaiOrder {
    pub order_id: i32,
    pub quantity: Text<Satoshis>,
    pub price: Text<Erc20Amount>,
}

impl InsertableBtcDaiOrder {
    pub fn new(order_fk: i32, quantity: asset::Bitcoin, price: Erc20Quantity) -> Self {
        Self {
            order_id: order_fk,
            quantity: Text(quantity.into()),
            price: Text(price.into()),
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(btc_dai_orders::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Order)]
#[table_name = "order_hbit_params"]
pub struct OrderHbitParams {
    id: i32,
    pub order_id: i32,
    pub network: Text<::bitcoin::Network>,
    pub side: Text<Side>,
    pub our_final_address: Text<::bitcoin::Address>,
    pub expiry_offset: i64,
}

impl OrderHbitParams {
    pub fn by_order(conn: &SqliteConnection, order: &Order) -> Result<Self> {
        let params = Self::belonging_to(order)
            .first::<Self>(conn)
            .with_context(|| format!("no hbit params found for order {}", order.order_id.0))?;

        Ok(params)
    }
}

#[derive(Insertable, Clone, Debug)]
#[table_name = "order_hbit_params"]
pub struct InsertableOrderHbitParams {
    pub order_id: i32,
    pub network: Text<::bitcoin::Network>,
    pub side: Text<Side>,
    pub our_final_address: Text<::bitcoin::Address>,
    pub expiry_offset: i64,
}

impl InsertableOrderHbitParams {
    pub fn new(
        order_fk: i32,
        network: ::bitcoin::Network,
        our_final_address: ::bitcoin::Address,
        expiry_offset: i64,
        side: Side,
    ) -> Self {
        InsertableOrderHbitParams {
            order_id: order_fk,
            network: Text(network),
            side: Text(side),
            our_final_address: Text(our_final_address),
            expiry_offset,
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(order_hbit_params::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Order)]
#[table_name = "order_herc20_params"]
pub struct OrderHerc20Params {
    id: i32,
    pub order_id: i32,
    pub chain_id: U32,
    pub side: Text<Side>,
    pub our_htlc_address: Text<ethereum::Address>,
    pub token_contract: Text<ethereum::Address>,
    pub expiry_offset: i64,
}

impl OrderHerc20Params {
    pub fn by_order(conn: &SqliteConnection, order: &Order) -> Result<Self> {
        let params = Self::belonging_to(order)
            .first::<Self>(conn)
            .with_context(|| format!("no herc20 params found for order {}", order.order_id.0))?;

        Ok(params)
    }
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "order_herc20_params"]
pub struct InsertableOrderHerc20Params {
    pub order_id: i32,
    pub chain_id: U32,
    pub side: Text<Side>,
    pub our_htlc_identity: Text<ethereum::Address>,
    pub token_contract: Text<ethereum::Address>,
    pub expiry_offset: i64,
}

impl InsertableOrderHerc20Params {
    pub fn new(
        order_fk: i32,
        chain_id: ChainId,
        our_htlc_identity: identity::Ethereum,
        token_contract: ethereum::Address,
        expiry_offset: i64,
        side: Side,
    ) -> Self {
        Self {
            order_id: order_fk,
            chain_id: u32::from(chain_id).into(),
            side: Text(side),
            our_htlc_identity: Text(our_htlc_identity),
            token_contract: Text(token_contract),
            expiry_offset,
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(order_herc20_params::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

/// A join table that tracks, which swaps resulted out of which order.
///
/// It is a common join-table naming convention to name these after the two
/// tables that are being associated: In our case, we are associating
/// potentially multiple swaps with a single order, hence the name "OrderSwaps".
#[derive(Associations, Clone, Copy, Debug, Queryable, PartialEq)]
#[belongs_to(Order)]
#[belongs_to(Swap)]
#[table_name = "order_swaps"]
pub struct OrderSwap {
    pub order_id: i32,
    pub swap_id: i32,
}

#[derive(Insertable, Clone, Copy, Debug)]
#[table_name = "order_swaps"]
pub struct InsertableOrderSwap {
    pub order_id: i32,
    pub swap_id: i32,
}

impl InsertableOrderSwap {
    pub fn new(swap_pk: i32, order_pk: i32) -> Self {
        Self {
            order_id: order_pk,
            swap_id: swap_pk,
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(order_swaps::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

impl From<Hbit> for asset::Bitcoin {
    fn from(hbit: Hbit) -> Self {
        hbit.amount.0.into()
    }
}

impl IntoInsertable for hbit::CreatedSwap {
    type Insertable = InsertableHbit;

    fn into_insertable(self, swap_id: i32, _: Role, side: Side) -> Self::Insertable {
        InsertableHbit {
            swap_id,
            amount: Text(self.amount.into()),
            network: Text(self.network.into()),
            expiry: U32(self.absolute_expiry),
            final_identity: Text(self.final_identity.into()),
            // We always retrieve the transient identity from the other party
            transient_identity: None,
            side: Text(side),
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

impl Insert<InsertableHalbit> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHalbit,
    ) -> anyhow::Result<()> {
        diesel::insert_into(halbits::dsl::halbits)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}

impl Insert<InsertableHbit> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHbit,
    ) -> anyhow::Result<()> {
        diesel::insert_into(hbits::dsl::hbits)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}

trait EnsureSingleRowAffected {
    fn ensure_single_row_affected(self) -> anyhow::Result<usize>;
}

impl EnsureSingleRowAffected for usize {
    fn ensure_single_row_affected(self) -> anyhow::Result<usize> {
        if self != 1 {
            return Err(anyhow::anyhow!(
                "Expected rows to be updated should have been 1 but was {}",
                self
            ));
        }
        Ok(self)
    }
}

impl Sqlite {
    pub fn insert_secret_hash(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        secret_hash: comit::SecretHash,
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

    pub fn update_halbit_refund_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Lightning,
    ) -> anyhow::Result<()> {
        diesel::update(halbits::table)
            .filter(halbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(halbits::refund_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update halbit refund identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_halbit_redeem_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Lightning,
    ) -> anyhow::Result<()> {
        diesel::update(halbits::table)
            .filter(halbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(halbits::redeem_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update halbit redeem identity for swap {}",
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
            .execute(connection)?
            .ensure_single_row_affected()
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
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update herc20 redeem identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_hbit_transient_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Bitcoin,
    ) -> anyhow::Result<()> {
        diesel::update(hbits::table)
            .filter(hbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(hbits::transient_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update hbit transient identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proptest::*;
    use proptest::prelude::*;
    use tokio::runtime::Runtime;

    proptest! {
        /// Verify that our database enforces foreign key relations
        ///
        /// We generate a random InsertableHalbit. This comes with a
        /// random swap_id already.
        /// We start with an empty database, so there is no swap that
        /// exists with this swap_id.
        #[test]
        fn fk_relations_are_enforced(
            insertable_halbit in db::tables::insertable_halbit(),
        ) {
            let db = Sqlite::test();
            let mut runtime = Runtime::new().unwrap();

            let result = runtime.block_on(db.do_in_transaction(|conn| db.insert(conn, &insertable_halbit)));

            result.unwrap_err();
        }
    }
}
