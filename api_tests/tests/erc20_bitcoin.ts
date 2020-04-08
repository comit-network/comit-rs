/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";

describe("E2E: Ethereum/erc20 - Bitcoin/bitcoin", () => {
    it(
        "rfc003-eth-erc20_btc-alice-redeems-bob-redeems",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Erc20, AssetKind.Bitcoin);
            await bob.accept();

            await alice.deploy();
            await alice.fund();
            await bob.fund();

            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        })
    );

    it(
        "rfc003-eth-erc20_btc-bob-refunds-alice-refunds",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Erc20, AssetKind.Bitcoin);
            await bob.accept();

            await alice.deploy();
            await alice.fund();
            await bob.fund();

            await alice.refund();
            await bob.refund();

            await alice.assertRefunded();
            await bob.assertRefunded();
        })
    );
});
