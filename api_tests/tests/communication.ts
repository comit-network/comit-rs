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
                .herc20EthereumErc20HalightLightningBitcoin;

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
                .herc20EthereumErc20HalightLightningBitcoin;

            await bob.createSwap(bodies.bob);
            await sleep(500);
            await alice.createSwap(bodies.alice);

            await assertSwapFinalized(alice);
            await assertSwapFinalized(bob);
        })
    );

    it(
        "swap-announced-with-wrong-peer-id-does-not-finalize",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob))
                .herc20EthereumErc20HalightLightningBitcoin;

            // Simulate that Bob is awaiting a swap from a different peer-id than Alice node's peer-id.
            bodies.bob.peer.peer_id =
                "QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N";
            await alice.createSwap(bodies.alice);
            await bob.createSwap(bodies.bob);

            await assertSwapCreated(alice);
            await assertSwapCreated(bob);

            await sleep(1000);

            // Assert that the swaps are still not finalized
            await assertSwapCreated(alice);
            await assertSwapCreated(bob);
        })
    );
});

/**
 * Assert that the swap has been created via the REST API bu tthe communication phase has not been finalized.
 *
 * No point adding it to `Actor` as it is only use in this test.
 */
async function assertSwapCreated(actor: Actor) {
    await actor.pollCndUntil(
        actor.swap.self,
        (swapResponse) => swapResponse.properties.status === SwapStatus.Created
    );
}

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
