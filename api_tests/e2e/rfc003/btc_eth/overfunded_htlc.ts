import { twoActorTest } from "../../../lib_sdk/actor_test";
import { AssetKind } from "../../../lib_sdk/asset";

setTimeout(function() {
    twoActorTest("rfc003-btc-eth-alice-overfunds-bob-aborts", async function({
        alice,
        bob,
    }) {
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
    });

    twoActorTest("rfc003-btc-eth-bob-overfunds-both-refund", async function({
        alice,
        bob,
    }) {
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
    });

    run();
}, 0);
