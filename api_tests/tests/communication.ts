/**
 * @ledger ethereum
 * @ledger lightning
 */

// ******************************************** //
// Test correct behaviour of the communication  //
// phase from creation to finalisation          //
// ******************************************** //

import { twoActorTest } from "../src/actor_test";
import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { SwapStatus } from "../src/swap_response";
import { Actor } from "../src/actors/actor";

describe("communication", () => {
    it(
        "Alice creates and then Bob creates, it finalizes",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob))
                .hanEthereumEtherHalightLightningBitcoin;

            await alice.createSwap(bodies.alice);
            await sleep(500);
            await bob.createSwap(bodies.bob);

            await assertSwapFinalized(alice);
            await assertSwapFinalized(bob);
        })
    );

    it(
        "Bob creates and then Alice creates, it finalizes",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob))
                .hanEthereumEtherHalightLightningBitcoin;

            await bob.createSwap(bodies.bob);
            await sleep(500);
            await alice.createSwap(bodies.alice);

            await assertSwapFinalized(alice);
            await assertSwapFinalized(bob);
        })
    );
});

/**
 * Assert that the communication phase has been finalized.
 *
 * No point adding it to `Actor` as it is only use in this test.
 */
async function assertSwapFinalized(actor: Actor) {
    await actor.pollCndUntil(
        actor.swap.self,
        (swapResponse) =>
            swapResponse.properties.status === SwapStatus.InProgress
    );
}
