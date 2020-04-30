/**
 * @ledger ethereum
 * @ledger lightning
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

it(
    "han-ethereum-ether-halight-lightning-bitcoin-alice-redeems-bob-redeems",
    twoActorTest(async ({ alice, bob }) => {
        const bodies = (await SwapFactory.newSwap(alice, bob))
            .hanEthereumEtherHalightLightningBitcoin;

        // Bob needs to know about the swap first because he is not buffering incoming announcements about swaps he doesn't know about
        await bob.createSwap(bodies.bob);
        await sleep(500);

        await alice.createSwap(bodies.alice);

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
        await bob.createSwap(bodies.bob);
        await sleep(500);

        await alice.createSwap(bodies.alice);

        const config = {
            maxTimeoutSecs: 1,
            tryIntervalSecs: 0.3,
        };

        // Due to the asynchronous nature of the announce protocol (and the fact that there is not error handling (yet)),
        // this test only tests that the "init" action does not become available.
        //
        // Scenario details:
        // 0) Alice and Bob agree on the swap params (negotiation); Alice gives Bob a peer-id that is not her node's peer-id.
        // 1) Bob posts swap, retrieves local swap-ID
        // 2) Alice posts swap, retrieves local swap-ID
        //      2.1) Internally the announce protocol runs into an Error because Alice' peer-id does not match the one she gave Bob.
        //              This Error, is however not visible to the outside.
        //              The application is responsible for removing swaps that do not move forward after a given time.
        const aliceResponsePromise = alice.init(config);
        return expect(aliceResponsePromise).rejects.toThrowError();
        // Same for Bob's side
        const bobResponsePromise = bob.fund(config);
        return expect(bobResponsePromise).rejects.toThrowError();
    })
);
