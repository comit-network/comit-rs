/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import OrderbookUtils from "../src/actors/order_factory";
import { sleep } from "../src/utils";

describe("orderbook", () => {
    it(
        "btc_dai_sell_order",
        twoActorTest(async ({ alice, bob }) => {
            await OrderbookUtils.connect(alice, bob);
            await OrderbookUtils.initialiseWalletsForBtcDaiOrder(alice, bob);

            const order = await OrderbookUtils.newBtcDaiSellOrder(bob);
            await alice.initLedgerAndBalancesForOrder(order);
            await bob.initLedgerAndBalancesForOrder(order);

            const aliceIdentities = await OrderbookUtils.getIdentities(alice);

            const orderUrl = await bob.makeOrder(order);

            await alice.takeOrderAndAssertSwapCreated(
                aliceIdentities.ethereum,
                aliceIdentities.bitcoin
            );

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
