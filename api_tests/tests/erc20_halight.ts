/**
 * @ledger ethereum
 * @ledger lightning
 */

import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import { twoActorTest } from "../src/actor_test";

it(
    "herc20-ethereum-erc20-halight-lightning-bitcoin-alice-redeems-bob-redeems",
    twoActorTest(async ({ alice, bob }) => {
        const bodies = (await SwapFactory.newSwap(alice, bob))
            .herc20EthereumErc20HalightLightningBitcoin;

        await alice.createHerc20HalightSwap(bodies.alice);
        await bob.createHerc20HalightSwap(bodies.bob);

        await alice.init();

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
