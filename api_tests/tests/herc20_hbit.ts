/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import SwapFactory from "../src/swap_factory";
import { sleep } from "../src/utils";
import { startAliceAndBob } from "../src/actor_test";

describe("herc20-hbit", () => {
    it(
        "herc20-hbit-alice-redeems-bob-redeems",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).herc20Hbit;

            await alice.createHerc20HbitSwap(bodies.alice);
            await bob.createHerc20HbitSwap(bodies.bob);

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

    it(
        "herc20-hbit-bob-refunds-alice-refunds",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    instantRefund: true,
                })
            ).herc20Hbit;

            await alice.createHerc20HbitSwap(bodies.alice);
            await bob.createHerc20HbitSwap(bodies.bob);

            await alice.assertAndExecuteNextAction("deploy");
            await alice.assertAndExecuteNextAction("fund");

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
