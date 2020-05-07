/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";
import { Rfc003Actor } from "../src/actors/rfc003_actor";

describe("E2E: Ethereum/erc20 - Bitcoin/bitcoin", () => {
    it(
        "rfc003-eth-erc20_btc-alice-redeems-bob-redeems",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
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
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
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
