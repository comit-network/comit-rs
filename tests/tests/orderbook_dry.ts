/**
 * @cndConfigOverride ethereum.chain_id = 1337
 * @cndConfigOverride ethereum.tokens.dai = 0x0000000000000000000000000000000000000000
 */
import { startAlice, startConnectedAliceAndBob } from "../src/actor_test";
import {
    Currency,
    MarketEntity,
    OrderEntity,
    Position,
} from "../src/cnd_client/payload";
import { Problem } from "../src/axios_rfc7807_middleware";

test(
    "given_two_connected_nodes_when_other_node_publishes_order_then_it_is_returned_in_the_market",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);
        const bobsPeerId = await bob.cnd.getPeerId();

        const aliceMarket = await alice.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length > 0
        );
        const bobMarket = await bob.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length > 0
        );

        const expectedOrder = {
            position: Position.Sell,
            maker: bobsPeerId,
            quantity: {
                currency: "BTC",
                value: "20000000",
                decimals: 8,
            },
            price: {
                currency: "DAI",
                value: "9000000000000000000000",
                decimals: 18,
            },
        };
        expect(aliceMarket.entities[0].properties).toMatchObject({
            ...expectedOrder,
            ours: false,
        });
        expect(bobMarket.entities[0].properties).toMatchObject({
            ...expectedOrder,
            ours: true, // Bob should see his own order in the market
        });
    })
);

test(
    "given_i_make_an_order_when_i_restart_my_node_it_should_still_be_there",
    startAlice(async (alice) => {
        const href = await alice.makeBtcDaiOrder(Position.Sell, 0.2, 9000);

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
    })
);

test(
    "given_alice_makes_an_order_when_fully_matched_against_bobs_order_then_settling_says_quantity",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        const aliceHref = await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
        const bobHref = await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);

        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        await expect(
            alice.fetchOrder(aliceHref).then((r) => r.properties)
        ).resolves.toMatchObject({
            state: {
                open: "0",
                settling: "20000000",
            },
        });
        await expect(
            bob.fetchOrder(bobHref).then((r) => r.properties)
        ).resolves.toMatchObject({
            state: {
                open: "0",
                settling: "20000000",
            },
        });
    })
);

test(
    "given_alice_makes_an_order_when_listing_all_orders_then_it_is_returned",
    startAlice(async (alice) => {
        await alice.makeBtcDaiOrder(Position.Sell, 0.2, 9000);

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
    })
);

test(
    "given_a_settling_order_when_open_orders_are_listed_is_still_returned_but_cannot_be_cancelled",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
        await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);
        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        const orders = await alice.listOpenOrders();

        expect(orders.entities).toHaveLength(1);
        expect(orders.entities[0].actions).toHaveLength(0);
    })
);

test(
    "given_a_settling_order_when_open_orders_are_listed_is_still_returned_but_cannot_be_cancelled",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
        await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);
        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        const orders = await alice.listOpenOrders();

        expect(orders.entities).toHaveLength(1);
        expect(orders.entities[0].actions).toHaveLength(0);
    })
);

test(
    "given_an_order_when_cancelled_state_changes_to_cancelled",
    startAlice(async (alice) => {
        const href = await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);

        const order = await alice.fetchOrder(href);

        expect(order.actions).toHaveLength(1);
        await alice.executeSirenAction(order, "cancel");

        await expect(
            alice.fetchOrder(href).then((r) => r.properties)
        ).resolves.toMatchObject({
            state: {
                open: "0",
                cancelled: "20000000",
            },
        });
        await expect(
            alice.fetchOrder(href).then((r) => r.actions)
        ).resolves.toHaveLength(0);
    })
);

test(
    "given_a_settling_order_when_trying_to_cancel_then_fails",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        const href = await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
        await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);
        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        const order = await alice.fetchOrder(href);
        const cancelAttempt = alice.cnd.client.delete(
            `/orders/${order.properties.id}`
        );

        await expect(cancelAttempt).rejects.toEqual(
            new Problem({
                status: 400,
                title: "Order can no longer be cancelled.",
            })
        );
    })
);

test(
    "given_an_order_when_cancelled_then_it_is_taken_from_the_market",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        // make an order and wait until Bob sees it
        const href = await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
        await bob.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length > 0
        );

        // cancel it
        const order = await alice.fetchOrder(href);
        await alice.executeSirenAction(order, "cancel");

        // assert that bob no longer sees the order
        await bob.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length === 0
        );
    })
);

test(
    "given_an_order_when_it_fully_matches_and_swap_is_setup_then_order_is_removed_from_the_market",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
        await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);

        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        await bob.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length === 0
        );
    })
);

test(
    "given_an_order_when_cancelled_then_it_is_no_longer_returned_in_open_orders",
    startAlice(async (alice) => {
        const href = await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);

        const order = await alice.fetchOrder(href);
        await alice.executeSirenAction(order, "cancel");

        await expect(
            alice.listOpenOrders().then((o) => o.entities)
        ).resolves.toHaveLength(0);
    })
);
