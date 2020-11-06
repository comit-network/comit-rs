/**
 * @cndConfigOverride ethereum.chain_id = 1337
 * @cndConfigOverride ethereum.tokens.dai = 0x0000000000000000000000000000000000000000
 */
import { startAlice } from "../src/actor_test";
import { Currency, OrderEntity, Position } from "../src/cnd_client/payload";

test(
    "given_i_make_an_order_when_i_restart_my_node_it_should_still_be_there",
    startAlice(async (alice) => {
        const href = await alice.makeBtcDaiOrder(Position.Sell, "0.2", "9000");

        await alice.restart();

        const response = await alice.cnd.fetch<OrderEntity>(href);
        expect(response.data.properties).toMatchObject({
            position: Position.Sell,
            quantity: {
                currency: Currency.BTC,
                value: "20000000",
                decimals: 8,
            },
            price: {
                currency: Currency.DAI,
                value: "9000000000000000000000",
                decimals: 18,
            },
        });
    }),
);

test(
    "given_alice_makes_an_order_when_listing_all_orders_then_it_is_returned",
    startAlice(async (alice) => {
        await alice.makeBtcDaiOrder(Position.Sell, "0.2", "9000");

        const orders = await alice.listOpenOrders();

        expect(orders.entities).toHaveLength(1);
        expect(orders.entities[0].properties).toMatchObject({
            position: Position.Sell,
            quantity: {
                currency: Currency.BTC,
                value: "20000000",
                decimals: 8,
            },
            price: {
                currency: Currency.DAI,
                value: "9000000000000000000000",
                decimals: 18,
            },
            state: {
                open: "20000000",
            },
        });
    }),
);

test(
    "given_an_order_when_cancelled_state_changes_to_cancelled",
    startAlice(async (alice) => {
        const href = await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");

        const order = await alice.fetchOrder(href);

        expect(order.actions).toHaveLength(1);
        await alice.executeSirenAction(order, "cancel");

        await expect(
            alice.fetchOrder(href).then((r) => r.properties),
        ).resolves.toMatchObject({
            state: {
                open: "0",
                cancelled: "20000000",
            },
        });
        await expect(
            alice.fetchOrder(href).then((r) => r.actions),
        ).resolves.toHaveLength(0);
    }),
);

test(
    "given_an_order_when_cancelled_then_it_is_no_longer_returned_in_open_orders",
    startAlice(async (alice) => {
        const href = await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");

        const order = await alice.fetchOrder(href);
        await alice.executeSirenAction(order, "cancel");

        await expect(
            alice.listOpenOrders().then((o) => o.entities),
        ).resolves.toHaveLength(0);
    }),
);

test(
    "given_an_order_when_cnd_is_restarted_then_the_order_is_republished_to_the_market",
    startAlice(async (alice) => {
        await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");
        const btcDaiMarket1 = await alice.getBtcDaiMarket();
        expect(btcDaiMarket1.entities).toHaveLength(1);

        await alice.restart();

        const btcDaiMarket2 = await alice.getBtcDaiMarket();
        expect(btcDaiMarket2.entities).toHaveLength(1);
    }),
);
