/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";

describe("E2E: Bitcoin/bitcoin - Ethereum/erc20", () => {
    it(
        "rfc003-btc-eth-erc20-alice-redeems-bob-redeems",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Erc20);
            await bob.accept();

            await alice.fund();
            await bob.deploy();
            await bob.fund();

            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        })
    );

    it(
        "rfc003-btc-eth-erc20-bob-refunds-alice-refunds",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Erc20);
            await bob.accept();

            await alice.fund();
            await bob.deploy();
            await bob.fund();

            await alice.refund();
            await bob.refund();

            await alice.assertRefunded();
            await bob.assertRefunded();
        })
    );
});
