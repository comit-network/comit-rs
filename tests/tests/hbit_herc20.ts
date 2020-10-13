/**
 * @ledger bitcoin
 * @ledger ethereum
 */

import SwapFactory from "../src/swap_factory";
import { sleep } from "../src/utils";
import { startAliceAndBob } from "../src/actor_test";

describe("hbit-herc20", () => {
    it(
        "hbit-herc20-alice-redeems-bob-redeems",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).hbitHerc20;

            await alice.createHbitHerc20Swap(bodies.alice);
            await bob.createHbitHerc20Swap(bodies.bob);

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
        "hbit-herc20-bob-refunds-alice-refunds",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    instantRefund: true,
                })
            ).hbitHerc20;

            await alice.createHbitHerc20Swap(bodies.alice);
            await bob.createHbitHerc20Swap(bodies.bob);

            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("deploy");
            await bob.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("refund");
            await alice.assertAndExecuteNextAction("refund");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterRefund();
            await bob.assertBalancesAfterRefund();
        })
    );
});
