/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import OrderFactory from "../src/actors/order_factory";
import { sleep } from "../src/utils";

// todo: move test initialisation into single mega function to reduce noise
describe("orderbook", () => {
    it(
        "btc_dai_buy_order",
        twoActorTest(async ({ alice, bob }) => {
            await alice.connect(bob);
            const order = await OrderFactory.newBtcDaiOrder(alice, bob, "buy");

            const orderUrl = await bob.makeOrder(order);
            await alice.takeOrder();

            await bob.checkSwapCreatedFromOrder(orderUrl);

            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("deploy");
            await bob.assertAndExecuteNextAction("fund");

            await alice.assertAndExecuteNextAction("redeem");
            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );
    it(
        "btc_dai_sell_order",
        twoActorTest(async ({ alice, bob }) => {
            await alice.connect(bob);
            const order = await OrderFactory.newBtcDaiOrder(alice, bob, "sell");

            const orderUrl = await bob.makeOrder(order);
            await alice.takeOrder();

            await bob.checkSwapCreatedFromOrder(orderUrl);

            await alice.assertAndExecuteNextAction("deploy");
            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("fund");

            await alice.assertAndExecuteNextAction("redeem");
            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );
});
