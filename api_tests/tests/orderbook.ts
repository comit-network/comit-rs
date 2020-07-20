/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { sleep } from "../src/utils";
import SwapFactory from "../src/actors/swap_factory";

describe("orderbook", () => {
    it(
        "btc_dai_sell_order",
        twoActorTest(async ({ alice, bob }) => {
            // Get alice's listen address
            const aliceAddr = await alice.cnd.getPeerListenAddresses();

            // Bob dials alices
            // @ts-ignore
            await bob.cnd.client.post("dial", { addresses: aliceAddr });

            /// Wait for alice to accept an incoming connection from Bob
            await sleep(1000);

            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "ethereum",
                        beta: "bitcoin",
                    },
                })
            ).herc20Hbit;

            const orderUrl = await bob.makeOrder(bodies.bob);
            await alice.takeOrderAndAssertSwapCreated(bodies.alice);

            await bob.assertSwapCreatedFromOrder(orderUrl, bodies.bob);

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
