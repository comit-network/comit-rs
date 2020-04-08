/**
 * @ledger ethereum
 * @ledger lightning
 */

import { twoActorTest } from "../src/actor_test";
import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";

it(
    "han-ethereum-ether-halight-lightning-bitcoin-alice-redeems-bob-redeems",
    twoActorTest(async ({ alice, bob }) => {
        const [aliceBody, bobBody] = await SwapFactory.newSwap(alice, bob);

        // Bob needs to know about the swap first because he is not buffering incoming announcements about swaps he doesn't know about
        await bob.createSwap(bobBody);
        await sleep(500);

        await alice.createSwap(aliceBody);

        await alice.init();

        await alice.fund();

        // we must not wait for bob's funding because `sendpayment` on a hold-invoice is a blocking call.
        // tslint:disable-next-line:no-floating-promises
        bob.fund();

        await alice.redeem();
        await bob.redeem();

        await sleep(2000);

        await alice.assertBalances();
        await bob.assertBalances();
    })
);
