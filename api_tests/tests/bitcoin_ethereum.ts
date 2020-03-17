/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";
import { sleep } from "../src/utils";

// ******************************************** //
// Bitcoin/bitcoin Alpha Ledger/Alpha Asset     //
// Ethereum/ether Beta Ledger/Beta Asset        //
// ******************************************** //
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

    // ************************ //
    // Refund test              //
    // ************************ //

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
    // Restart cnd test         //
    // ************************ //

    it(
        "rfc003-btc-eth-cnd-can-be-restarted",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.currentSwapIsAccepted();
            await bob.currentSwapIsAccepted();

            await alice.restart();
            await bob.restart();

            await alice.currentSwapIsAccepted();
            await bob.currentSwapIsAccepted();
        })
    );

    // ************************ //
    // Resume cnd test          //
    // ************************ //

    it(
        "rfc003-btc-eth-resume-alice-down-bob-funds",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await alice.stop();

            // Action happens while alice is down.
            await bob.fund();

            // Blocks are geneated every second here, wait to ensure
            // we look into the past for the transaction.
            await sleep(2000);
            await alice.start();

            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        })
    );

    it(
        "rfc003-btc-eth-resume-alice-down-bob-redeems",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await alice.redeem();
            await alice.stop();

            // Action happens while alice is down.
            await bob.redeem();

            // Blocks are geneated every second here, wait to ensure
            // we look into the past for the transaction.
            await sleep(2000);
            await alice.start();

            await alice.assertSwapped();
            await bob.assertSwapped();
        })
    );

    it(
        "rfc003-btc-eth-resume-bob-down-alice-funds",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            // Wait for Alice to receive the accept message before stopping Bob's cnd.
            await alice.currentSwapIsAccepted();

            await bob.stop();

            // Action happens while bob is down.
            await alice.fund();

            // Blocks are geneated every second here, wait to ensure
            // we look into the past for the transaction.
            await sleep(2000);
            await bob.start();

            await bob.fund();

            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        })
    );

    it(
        "rfc003-btc-eth-resume-bob-down-alice-redeems",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await bob.stop();

            // Action happens while bob is down.
            await alice.redeem();

            // Blocks are geneated every second here, wait to ensure
            // we look into the past for the transaction.
            await sleep(2000);
            await bob.start();

            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
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

// ******************************************** //
// Bitcoin/bitcoin Alpha Ledger/ Alpha Asset    //
// Ethereum/erc20 Beta Ledger/Beta Asset        //
// ******************************************** //
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
