/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { AssetKind } from "../src/asset";
import { sleep } from "../src/utils";

describe("E2E: Bitcoin/bitcoin - Ethereum/ether (restart cnd nodes)", () => {
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
});
