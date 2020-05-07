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

describe("communication", () => {
    it(
        "Alice creates and then Bob creates, it finalizes",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob))
                .hanEthereumEtherHalightLightningBitcoin;

            await alice.createSwap(bodies.alice);
            await sleep(500);
            await bob.createSwap(bodies.bob);

            await alice.assertSwapFinalized();
            await bob.assertSwapFinalized();
        })
    );

    it(
        "Bob creates and then Alice creates, it finalizes",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob))
                .hanEthereumEtherHalightLightningBitcoin;

            await bob.createSwap(bodies.alice);
            await sleep(500);
            await alice.createSwap(bodies.bob);

            await alice.assertSwapFinalized();
            await bob.assertSwapFinalized();
        })
    );
});
