/**
 * @ledger ethereum
 * @ledger lightning
 */

import SwapFactory from "../src/swap_factory";
import { sleep } from "../src/utils";
import { startAliceAndBob } from "../src/actor_test";

describe("herc20-halbit", () => {
    it(
        "herc20-halbit-alice-redeems-bob-redeems",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).herc20Halbit;
            await bob.openLnChannel(alice, bodies.bob.beta.amount * 2n);

            await alice.createHerc20HalbitSwap(bodies.alice);
            await bob.createHerc20HalbitSwap(bodies.bob);

            await alice.assertAndExecuteNextAction("init");

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
        "herc20-halbit-alice-refunds",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    instantRefund: true,
                })
            ).herc20Halbit;
            await bob.openLnChannel(alice, bodies.bob.beta.amount * 2n);

            await alice.createHerc20HalbitSwap(bodies.alice);
            await bob.createHerc20HalbitSwap(bodies.bob);

            await alice.assertAndExecuteNextAction("init");

            await alice.assertAndExecuteNextAction("deploy");
            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("fund");

            await alice.assertAndExecuteNextAction("refund");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterRefund();
        })
    );
});
