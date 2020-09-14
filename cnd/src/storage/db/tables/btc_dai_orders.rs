use crate::{
    asset,
    storage::{
        db::{
            schema::*,
            wrapper_types::{Erc20Amount, Satoshis, WeiPerSat},
        },
        NotOpen, Order, Text,
    },
};
use anyhow::{Context, Result};
use comit::{
    asset::{bitcoin, ethereum::Erc20Quantity},
    order::Quantity,
    Price,
};
use diesel::{prelude::*, sqlite::SqliteConnection};

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Order)]
#[table_name = "btc_dai_orders"]
pub struct BtcDaiOrder {
    id: i32,
    pub order_id: i32,
    #[diesel(deserialize_as = "Text<Satoshis>")]
    pub quantity: Quantity<bitcoin::Bitcoin>,
    #[diesel(deserialize_as = "Text<WeiPerSat>")]
    pub price: Price<bitcoin::Bitcoin, Erc20Quantity>,
    #[diesel(deserialize_as = "Text<Satoshis>")]
    pub open: Quantity<bitcoin::Bitcoin>,
    #[diesel(deserialize_as = "Text<Satoshis>")]
    pub closed: Quantity<bitcoin::Bitcoin>,
    #[diesel(deserialize_as = "Text<Satoshis>")]
    pub settling: Quantity<bitcoin::Bitcoin>,
    #[diesel(deserialize_as = "Text<Satoshis>")]
    pub failed: Quantity<bitcoin::Bitcoin>,
    #[diesel(deserialize_as = "Text<Satoshis>")]
    pub cancelled: Quantity<bitcoin::Bitcoin>,
}

impl BtcDaiOrder {
    pub fn by_order(conn: &SqliteConnection, order: &Order) -> Result<Self> {
        let params = Self::belonging_to(order)
            .first::<Self>(conn)
            .with_context(|| format!("order {} is not a BTC/DAI order", order.order_id))?;

        Ok(params)
    }

    /// Move the amount that is settling from open to settling.
    ///
    /// Whilst we don't have partial order matching, this simply means updating
    /// `settling` to the amount of `open` and updating `open` to `0`.
    ///
    /// Once we implement partial order matching, this will need to get more
    /// sophisticated.
    pub fn set_to_settling(&self, conn: &SqliteConnection) -> Result<()> {
        let affected_rows = diesel::update(self)
            .set((
                btc_dai_orders::settling.eq(Text::<Satoshis>(self.open.to_inner().into())),
                btc_dai_orders::open.eq(Text::<Satoshis>(bitcoin::Bitcoin::ZERO.into())),
            ))
            .execute(conn)?;

        if affected_rows == 0 {
            anyhow::bail!("failed to mark order {} as settling", self.order_id)
        }

        Ok(())
    }

    pub fn set_to_cancelled(&self, conn: &SqliteConnection) -> Result<()> {
        if self.open == Quantity::new(bitcoin::Bitcoin::ZERO) {
            let order = Order::by_id(conn, self.order_id)?;
            anyhow::bail!(NotOpen(order.order_id))
        }

        let affected_rows = diesel::update(self)
            .set((
                btc_dai_orders::cancelled.eq(Text::<Satoshis>(self.open.to_inner().into())),
                btc_dai_orders::open.eq(Text::<Satoshis>(bitcoin::Bitcoin::ZERO.into())),
            ))
            .execute(conn)?;

        if affected_rows == 0 {
            anyhow::bail!("failed to mark order {} as cancelled", self.order_id)
        }

        Ok(())
    }
}

#[derive(Insertable, Clone, Debug)]
#[table_name = "btc_dai_orders"]
pub struct InsertableBtcDaiOrder {
    pub order_id: i32,
    pub quantity: Text<Satoshis>,
    pub price: Text<Erc20Amount>,
    open: Text<Satoshis>,
    closed: Text<Satoshis>,
    settling: Text<Satoshis>,
    failed: Text<Satoshis>,
    cancelled: Text<Satoshis>,
}

impl InsertableBtcDaiOrder {
    pub fn new(order_fk: i32, quantity: bitcoin::Bitcoin, price: Erc20Quantity) -> Self {
        Self {
            order_id: order_fk,
            quantity: Text(quantity.into()),
            price: Text(price.into()),
            open: Text(quantity.into()),
            closed: Text(bitcoin::Bitcoin::ZERO.into()),
            settling: Text(bitcoin::Bitcoin::ZERO.into()),
            failed: Text(bitcoin::Bitcoin::ZERO.into()),
            cancelled: Text(bitcoin::Bitcoin::ZERO.into()),
        }
    }

    pub fn insert(self, conn: &SqliteConnection) -> Result<()> {
        diesel::insert_into(btc_dai_orders::table)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

pub fn all_open_btc_dai_orders(conn: &SqliteConnection) -> Result<Vec<(Order, BtcDaiOrder)>> {
    let orders = orders::table
        .inner_join(btc_dai_orders::table)
        .filter(btc_dai_orders::open.ne(Text::<Satoshis>(asset::Bitcoin::ZERO.into())))
        .or_filter(btc_dai_orders::settling.ne(Text::<Satoshis>(asset::Bitcoin::ZERO.into())))
        .load::<(Order, BtcDaiOrder)>(conn)?;

    Ok(orders)
}
