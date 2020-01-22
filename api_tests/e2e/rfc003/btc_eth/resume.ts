import { sleep } from "../../../lib/util";
import { twoActorTest } from "../../../lib_sdk/actor_test";
import { AssetKind } from "../../../lib_sdk/asset";

setTimeout(function() {
    twoActorTest("rfc003-btc-eth-resume-alice-down-bob-funds", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
        await bob.accept();

        await alice.fund();
        alice.stop();

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
    });

    twoActorTest(
        "rfc003-btc-eth-resume-alice-down-bob-redeems",
        async function({ alice, bob }) {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await alice.redeem();
            alice.stop();

            // Action happens while alice is down.
            await bob.redeem();

            // Blocks are geneated every second here, wait to ensure
            // we look into the past for the transaction.
            await sleep(2000);
            await alice.start();

            await alice.assertSwapped();
            await bob.assertSwapped();
        }
    );

    twoActorTest("rfc003-btc-eth-resume-bob-down-alice-funds", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
        await bob.accept();

        // Wait for Alice to receive the accept message before stopping Bob's cnd.
        await alice.currentSwapIsAccepted();

        bob.stop();

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
    });

    twoActorTest(
        "rfc003-btc-eth-resume-bob-down-alice-redeems",
        async function({ alice, bob }) {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            bob.stop();

            // Action happens while bob is down.
            await alice.redeem();

            // Blocks are geneated every second here, wait to ensure
            // we look into the past for the transaction.
            await sleep(2000);
            await bob.start();

            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        }
    );

    run();
}, 120_000);
