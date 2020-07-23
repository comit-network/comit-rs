/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import OrderFactory from "../src/actors/order_factory";
import { sleep } from "../src/utils";

describe("orderbook", () => {
    // direct quote
    // pair: "BTC/DAI"
    // position: buy
    // rate: 100 (1000 wei for 1 satoshi)
    // amount: 900,000 satoshi BTC
    //
    // Bob wants BTC for DAI
    // Alice only has 20,000 satoshi
    // Alice submits a take request specifying 20,000 satoshi as the the quantity
    // The take request becomes a swap where Bob receives 20,000 satoshi for 2,000,000 wei
    it(
        "btc_dai_buy_order",
        twoActorTest(async ({ alice, bob }) => {
            await alice.connect(bob);
            const order = await OrderFactory.newBtcDaiOrder(
                alice,
                bob,
                "buy",
                100,
                "900000"
            );

            const orderUrl = await bob.makeOrder(order);

            await alice.takeOrder("20000");

            await bob.assertSwapCreatedFromOrder(orderUrl);

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
    // direct quote
    // pair: "BTC/DAI"
    // position: sell
    // rate: 100 (1000 wei for 1 satoshi)
    // amount: 900,000 satoshi BTC
    //
    // Bob wants DAI for BTC
    // Alice only has 20,000 satoshi
    // Alice submits a take request specifying 20,000 satoshi as the the quantity
    // The take request becomes a swap where Bob receives for 2,000,000 wei for  20,000 satoshi
    it(
        "btc_dai_sell_order",
        twoActorTest(async ({ alice, bob }) => {
            await alice.connect(bob);
            const order = await OrderFactory.newBtcDaiOrder(
                alice,
                bob,
                "buy",
                100,
                "900000"
            );

            const orderUrl = await bob.makeOrder(order);

            await alice.takeOrder("20000");

            await bob.assertSwapCreatedFromOrder(orderUrl);

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
});
