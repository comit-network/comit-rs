/**
 * @ledger ethereum
 * @ledger lightning
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

describe("herc20-halight-respawn", () => {
    it(
        "herc20-halight-alice-misses-bob-fund",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "ethereum",
                        beta: "lightning",
                    },
                })
            ).herc20Halight;

            await alice.createHerc20HalightSwap(bodies.alice);
            await bob.createHerc20HalightSwap(bodies.bob);

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
        "herc20-halight-bob-misses-alice-redeem",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "ethereum",
                        beta: "lightning",
                    },
                })
            ).herc20Halight;

            await alice.createHerc20HalightSwap(bodies.alice);
            await bob.createHerc20HalightSwap(bodies.bob);

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
        "halight-herc20-alice-misses-bob-deploy-and-fund",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "lightning",
                        beta: "ethereum",
                    },
                })
            ).halightHerc20;

            await alice.createHalightHerc20Swap(bodies.alice);
            await bob.createHalightHerc20Swap(bodies.bob);

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
        "halight-herc20-bob-down-misses-alice-redeem",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "lightning",
                        beta: "ethereum",
                    },
                })
            ).halightHerc20;

            await alice.createHalightHerc20Swap(bodies.alice);
            await bob.createHalightHerc20Swap(bodies.bob);

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
