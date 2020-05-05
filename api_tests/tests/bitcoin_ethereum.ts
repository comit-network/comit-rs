/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";

describe("E2E: Bitcoin/bitcoin - Ethereum/ether", () => {
    it(
        "rfc003-btc-eth-alice-redeems-bob-redeems",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        })
    );

    it(
        "rfc003-btc-eth-bob-refunds-alice-refunds",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await bob.refund();
            await alice.refund();

            await bob.assertRefunded();
            await alice.assertRefunded();
        })
    );

    it(
        "rfc003-btc-eth-alice-refunds-bob-refunds",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await alice.refund();
            await bob.refund();

            await alice.assertRefunded();
            await bob.assertRefunded();
        })
    );

    // ************************ //
    // Underfunding test        //
    // ************************ //

    it(
        "rfc003-btc-eth-alice-underfunds-bob-aborts",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.underfund();

            await bob.assertAlphaIncorrectlyFunded();
            await bob.assertBetaNotDeployed();
            await alice.assertAlphaIncorrectlyFunded();
            await alice.assertBetaNotDeployed();

            await alice.refund();
            await alice.assertRefunded();

            await bob.assertBetaNotDeployed();
        })
    );

    it(
        "rfc003-btc-eth-bob-underfunds-both-refund",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();
            await alice.fund();

            await bob.assertAlphaFunded();
            await alice.assertAlphaFunded();

            await bob.underfund();

            await alice.assertBetaIncorrectlyFunded();
            await bob.assertBetaIncorrectlyFunded();

            await bob.refund();
            await bob.assertRefunded();
            await alice.refund();
            await alice.assertRefunded();
        })
    );

    // ************************ //
    // Overfund test            //
    // ************************ //

    it(
        "rfc003-btc-eth-alice-overfunds-bob-aborts",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.overfund();

            await bob.assertAlphaIncorrectlyFunded();
            await bob.assertBetaNotDeployed();
            await alice.assertAlphaIncorrectlyFunded();
            await alice.assertBetaNotDeployed();

            await alice.refund();
            await alice.assertRefunded();

            await bob.assertBetaNotDeployed();
        })
    );

    it(
        "rfc003-btc-eth-bob-overfunds-both-refund",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();
            await alice.fund();

            await bob.assertAlphaFunded();
            await alice.assertAlphaFunded();

            await bob.overfund();

            await alice.assertBetaIncorrectlyFunded();
            await bob.assertBetaIncorrectlyFunded();

            await bob.refund();
            await bob.assertRefunded();
            await alice.refund();
            await alice.assertRefunded();
        })
    );
});
