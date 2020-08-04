/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { sleep } from "../src/utils";
import OrderbookFactory from "../src/actors/order_factory";

describe("orderbook", () => {
    // pair: "BTC/DAI"
    // position: buy
    // price: 11003.46 (1 BTC = 11003.46 DAI)
    // quantity: 11.4 BTC
    //
    // Bob is buying BTC for DAI
    // Alice only has 2.1 BTC
    // Alice submits a take request specifying 2.1 BTC as the the quantity
    // The take request becomes a HbitHerc20 swap where Bob receives for 2.1 BTC for 23,107.266 DAI
    it(
        "btc_dai_buy_order",
        twoActorTest(async ({ alice, bob }) => {
            await alice.connect(bob);
            const order = await OrderbookFactory.newBtcDaiOrder(
                alice,
                bob,
                "buy",
                "9000.35",
                "0.4"
            );

            const orderUrl = await bob.makeOrder(order);

            await alice.takeOrder("0.1");

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
    // pair: "BTC/DAI"
    // position: sell
    // price: 11003.46 (1 BTC = 11003.46 DAI)
    // quantity: 11.4 BTC
    //
    // Bob is selling BTC for DAI
    // Alice only has 2.1 BTC
    // Alice submits a take request specifying 2.1 BTC as the the quantity
    // The take request becomes a Herc20Hbit swap where Bob receives 23,107.266 DAI for 2.1 BTC
    it(
        "btc_dai_sell_order",
        twoActorTest(async ({ alice, bob }) => {
            await alice.connect(bob);
            const order = await OrderbookFactory.newBtcDaiOrder(
                alice,
                bob,
                "sell",
                "9000.35",
                "0.01"
            );

            const orderUrl = await bob.makeOrder(order);

            await alice.takeOrder("0.005");

            await bob.assertSwapCreatedFromOrder(orderUrl);

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
