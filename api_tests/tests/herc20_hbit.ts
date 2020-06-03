/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

it(
    "herc20-hbit-alice-redeems-bob-redeems",
    twoActorTest(async ({ alice, bob }) => {
        const bodies = (
            await SwapFactory.newSwap(alice, bob, {
                alpha: "ethereum",
                beta: "bitcoin",
            })
        ).herc20Hbit;

        await alice.createHerc20HbitSwap(bodies.alice);
        await bob.createHerc20HbitSwap(bodies.bob);

        await alice.deploy();
        await alice.fund();

        await bob.fund();

        await alice.redeem();
        await bob.redeem();

        // Wait until the wallet sees the new balance.
        await sleep(2000);

        await alice.assertBalances();
        await bob.assertBalances();
    })
);
