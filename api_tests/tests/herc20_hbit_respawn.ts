/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

describe("herc20-hbit-respawn", () => {
    it(
        "herc20-hbit-alice-misses-bob-funds",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).herc20Hbit;

            await alice.createHerc20HbitSwap(bodies.alice);
            await bob.createHerc20HbitSwap(bodies.bob);

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
        "herc20-hbit-bob-misses-alice-redeems",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).herc20Hbit;

            await alice.createHerc20HbitSwap(bodies.alice);
            await bob.createHerc20HbitSwap(bodies.bob);

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
        "hbit-herc20-alice-misses-bob-deploys-and-funds",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).hbitHerc20;

            await alice.createHbitHerc20Swap(bodies.alice);
            await bob.createHbitHerc20Swap(bodies.bob);

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
        "hbit-herc20-bob-down-misses-alice-redeems",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).hbitHerc20;

            await alice.createHbitHerc20Swap(bodies.alice);
            await bob.createHbitHerc20Swap(bodies.bob);

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
