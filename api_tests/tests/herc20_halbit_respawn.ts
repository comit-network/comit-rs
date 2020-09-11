/**
 * @ledger ethereum
 * @ledger lightning
 */

import SwapFactory from "../src/swap_factory";
import { sleep } from "../src/utils";
import { startAliceAndBob } from "../src/actor_test";

describe("herc20-halbit-respawn", () => {
    it(
        "herc20-halbit-alice-misses-bob-fund",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).herc20Halbit;
            await bob.openLnChannel(alice, bodies.bob.beta.amount * 2n);

            await alice.createHerc20HalbitSwap(bodies.alice);
            await bob.createHerc20HalbitSwap(bodies.bob);

            await alice.assertAndExecuteNextAction("init");
            await alice.assertAndExecuteNextAction("deploy");
            await alice.assertAndExecuteNextAction("fund");

            await alice.stop();

            // Action happens while alice is down
            await bob.assertAndExecuteNextAction("fund");
            await sleep(2000);

            await alice.start();

            await alice.assertAndExecuteNextAction("redeem");
            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );

    it(
        "herc20-halbit-bob-misses-alice-redeem",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).herc20Halbit;
            await bob.openLnChannel(alice, bodies.bob.beta.amount * 2n);

            await alice.createHerc20HalbitSwap(bodies.alice);
            await bob.createHerc20HalbitSwap(bodies.bob);

            await alice.assertAndExecuteNextAction("init");
            await alice.assertAndExecuteNextAction("deploy");
            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("fund");

            await bob.stop();

            // Action happens while bob is down
            await alice.assertAndExecuteNextAction("redeem");
            await sleep(2000);

            await bob.start();

            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );

    it(
        "halbit-herc20-alice-misses-bob-deploy-and-fund",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).halbitHerc20;
            await alice.openLnChannel(bob, bodies.alice.alpha.amount * 2n);

            await alice.createHalbitHerc20Swap(bodies.alice);
            await bob.createHalbitHerc20Swap(bodies.bob);

            await bob.assertAndExecuteNextAction("init");

            await alice.assertAndExecuteNextAction("fund");

            await alice.stop();

            // Actions happen while alice is down
            await bob.assertAndExecuteNextAction("deploy");
            await bob.assertAndExecuteNextAction("fund");
            await sleep(2000);

            await alice.start();

            await alice.assertAndExecuteNextAction("redeem");
            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );

    it(
        "halbit-herc20-bob-down-misses-alice-redeem",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).halbitHerc20;
            await alice.openLnChannel(bob, bodies.alice.alpha.amount * 2n);

            await alice.createHalbitHerc20Swap(bodies.alice);
            await bob.createHalbitHerc20Swap(bodies.bob);

            await bob.assertAndExecuteNextAction("init");

            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("deploy");
            await bob.assertAndExecuteNextAction("fund");

            await bob.stop();

            // Action happens while bob is down
            await alice.assertAndExecuteNextAction("redeem");
            await sleep(2000);

            await bob.start();

            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );
});
