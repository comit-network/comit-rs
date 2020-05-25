/**
 * @ledger lightning
 * @ledger ethereum
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

it(
    "halight-lightning-bitcoin-herc20-ethereum-erc20-alice-redeems-bob-redeems",
    twoActorTest(async ({ alice, bob }) => {
        const bodies = (await SwapFactory.newSwap(alice, bob))
            .halightLightningBitcoinHerc20EthereumErc20;

        await alice.createHalightHerc20Swap(bodies.alice);
        await sleep(500);
        await bob.createHalightHerc20Swap(bodies.bob);

        await bob.init();

        // we must not wait for bob's funding because `sendpayment` on a hold-invoice is a blocking call.
        // tslint:disable-next-line:no-floating-promises
        alice.fund();

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
