/**
 * @cndConfigOverride ethereum.chain_id = 1337
 * @cndConfigOverride ethereum.tokens.dai = 0x0000000000000000000000000000000000000000
 */
import { startConnectedAliceAndBob } from "../src/actor_test";
import { MarketEntity, Position } from "../src/cnd_client/payload";
import { Problem } from "../src/axios_rfc7807_middleware";

test(
    "given_alice_makes_an_order_when_fully_matched_against_bobs_order_then_settling_says_quantity",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        const aliceHref = await alice.makeBtcDaiOrder(
            Position.Buy,
            "0.2",
            "9000"
        );
        const bobHref = await bob.makeBtcDaiOrder(Position.Sell, "0.2", "9000");

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
    "given_a_settling_order_when_open_orders_are_listed_is_still_returned_but_cannot_be_cancelled",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");
        await bob.makeBtcDaiOrder(Position.Sell, "0.2", "9000");
        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        const orders = await alice.listOpenOrders();

        expect(orders.entities).toHaveLength(1);
        expect(orders.entities[0].actions).toHaveLength(0);
    })
);

test(
    "given_a_settling_order_when_open_orders_are_listed_is_still_returned_but_cannot_be_cancelled",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");
        await bob.makeBtcDaiOrder(Position.Sell, "0.2", "9000");
        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        const orders = await alice.listOpenOrders();

        expect(orders.entities).toHaveLength(1);
        expect(orders.entities[0].actions).toHaveLength(0);
    })
);

test(
    "given_a_settling_order_when_trying_to_cancel_then_fails",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        const href = await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");
        await bob.makeBtcDaiOrder(Position.Sell, "0.2", "9000");
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
        const href = await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");
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
        await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");
        await bob.makeBtcDaiOrder(Position.Sell, "0.2", "9000");

        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        await bob.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length === 0
        );
    })
);
