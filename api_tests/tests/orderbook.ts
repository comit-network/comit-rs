/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import SwapFactory from "../src/actors/swap_factory";
import { twoActorTest } from "../src/actor_test";
// import { sleep } from "../src/utils";

describe("orderbook", () => {
    it(
        "btc_dai_sell_order",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "ethereum",
                        beta: "bitcoin",
                    },
                })
            ).herc20Hbit;

            await bob.makeOrder(bodies.bob);
            await alice.takeOrder(bodies.alice);

            // await alice.assertAndExecuteNextAction("deploy");
            // await alice.assertAndExecuteNextAction("fund");

            // await bob.assertAndExecuteNextAction("fund");

            // await alice.assertAndExecuteNextAction("redeem");
            // await bob.assertAndExecuteNextAction("redeem");

            // // Wait until the wallet sees the new balance.
            // await sleep(2000);

            // await alice.assertBalancesAfterSwap();
            // await bob.assertBalancesAfterSwap();
        })
    );
});
