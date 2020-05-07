/**
 * @ledger ethereum
 * @ledger lightning
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";
import { Actor } from "../src/actors/actor";
import { SwapStatus } from "../src/swap_response";

it(
    "han-ethereum-ether-halight-lightning-bitcoin-alice-redeems-bob-redeems",
    twoActorTest(async ({ alice, bob }) => {
        const bodies = (await SwapFactory.newSwap(alice, bob))
            .hanEthereumEtherHalightLightningBitcoin;

        await alice.createSwap(bodies.alice);
        await sleep(500);
        await bob.createSwap(bodies.bob);

        await alice.init();

        await alice.fund();

        // we must not wait for bob's funding because `sendpayment` on a hold-invoice is a blocking call.
        // tslint:disable-next-line:no-floating-promises
        bob.fund();

        await alice.redeem();
        await bob.redeem();

        // Wait until the wallet sees the new balance.
        await sleep(2000);

        await alice.assertBalances();
        await bob.assertBalances();
    })
);

it(
    "han-ethereum-ether-halight-lightning-bitcoin-alice-announces-with-wrong-peer-id",
    twoActorTest(async ({ alice, bob }) => {
        const bodies = (await SwapFactory.newSwap(alice, bob))
            .hanEthereumEtherHalightLightningBitcoin;

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

// This test could be anywhere, it is just here because this endpoint was the first of the spilt protocols implemented.
it(
    "alice-cannot-create-two-identical-swaps",
    twoActorTest(async ({ alice, bob }) => {
        // arrange
        const bodies = (await SwapFactory.newSwap(alice, bob))
            .hanEthereumEtherHalightLightningBitcoin;

        await alice.createSwap(bodies.alice);
        await sleep(200);

        // act
        const response = alice.createSwap(bodies.alice);

        // assert
        await expect(response).rejects.toThrow("Swap already exists.");
    })
);

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
