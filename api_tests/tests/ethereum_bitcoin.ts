/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";
import { sleep } from "../src/utils";
import { Rfc003Actor } from "../src/actors/rfc003_actor";

describe("E2E: Ethereum/ether - Bitcoin/bitcoin", () => {
    it(
        "rfc003-eth-btc-alice-redeems-bob-redeems",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
            await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        })
    );

    // ************************ //
    // Ignore Failed ETH TX     //
    // ************************ //

    it(
        "rfc003-eth-btc-alpha-deploy-fails",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
            await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
            await bob.accept();

            await alice.fundLowGas("0x1b000");

            // It is meaningless to assert before cnd processes a new block
            await sleep(3000);
            await alice.assertAlphaNotDeployed();
            await bob.assertAlphaNotDeployed();
            await bob.assertBetaNotDeployed();
            await alice.assertBetaNotDeployed();
        })
    );

    // ************************ //
    // Refund tests             //
    // ************************ //

    it(
        "rfc003-eth-btc-bob-refunds-alice-refunds",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
            await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await bob.refund();
            await alice.refund();

            await bob.assertRefunded();
            await alice.assertRefunded();
        })
    );

    // ************************ //
    // Bitcoin High Fees        //
    // ************************ //

    it(
        "rfc003-eth-btc-alice-redeems-with-high-fee",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
            await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            const responsePromise = alice.redeemWithHighFee();

            return expect(responsePromise).rejects.toThrowError();
        })
    );
});
