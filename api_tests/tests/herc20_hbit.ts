/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

describe("herc20-hbit", () => {
    it(
        "herc20-hbit-alice-redeems-bob-redeems",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "ethereum",
                        beta: "bitcoin",
                    },
                })
            ).herc20Hbit;

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
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "ethereum",
                        beta: "bitcoin",
                    },
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
