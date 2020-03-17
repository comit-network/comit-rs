/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";

// ******************************************** //
// Ethereum/ether Alpha Ledger/ Alpha Asset     //
// Bitcoin/bitcoin Beta Ledger/Beta Asset       //
// ******************************************** //

describe("E2E: Ethereum/ether - Bitcoin/bitcoin", () => {
    it(
        "rfc003-eth-btc-alice-redeems-bob-redeems",
        twoActorTest(async ({ alice, bob }) => {
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
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
            await bob.accept();

            await alice.fundLowGas("0x1b000");

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
        twoActorTest(async ({ alice, bob }) => {
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
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            const responsePromise = alice.redeemWithHighFee();

            return expect(responsePromise).rejects.toThrowError();
        })
    );
});

// ******************************************** //
// Ethereum/erc20 Alpha Ledger/ Alpha Asset     //
// Bitcoin/bitcoin Beta Ledger/Beta Asset       //
// ******************************************** //
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
