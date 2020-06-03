/**
 * @ledger bitcoin
 * @ledger ethereum
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

it(
    "hbit-herc20-alice-redeems-bob-redeems",
    twoActorTest(async ({ alice, bob }) => {
        const bodies = (
            await SwapFactory.newSwap(alice, bob, {
                alpha: "bitcoin",
                beta: "ethereum",
            })
        ).hbitHerc20;

        await alice.createHbitHerc20Swap(bodies.alice);
        await bob.createHbitHerc20Swap(bodies.bob);

        await alice.fund();

        await bob.deploy();
        await bob.fund();

        await alice.redeem();
        await bob.redeem();

        // Wait until the wallet sees the new balance.
        await sleep(2000);

        await alice.assertBalances();
        await bob.assertBalances();
    })
);
