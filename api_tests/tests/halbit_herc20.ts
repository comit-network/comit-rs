/**
 * @ledger lightning
 * @ledger ethereum
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

describe("halbit-herc20", () => {
    it(
        "halbit-herc20-alice-redeems-bob-redeems",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).halbitHerc20;

            await alice.createHalbitHerc20Swap(bodies.alice);
            await bob.createHalbitHerc20Swap(bodies.bob);

            await bob.assertAndExecuteNextAction("init");

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
        "halbit-herc20-bob-refunds",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    instantRefund: true,
                })
            ).halbitHerc20;

            await alice.createHalbitHerc20Swap(bodies.alice);
            await bob.createHalbitHerc20Swap(bodies.bob);

            await bob.assertAndExecuteNextAction("init");

            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("deploy");
            await bob.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("refund");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await bob.assertBalancesAfterRefund();
        })
    );
});
