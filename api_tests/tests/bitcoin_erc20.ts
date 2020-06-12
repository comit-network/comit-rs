/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";
import { Rfc003Actor } from "../src/actors/rfc003_actor";

describe("E2E: Bitcoin/bitcoin - Ethereum/erc20", () => {
    it(
        "rfc003-btc-eth-erc20-alice-redeems-bob-redeems",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);

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
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
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
