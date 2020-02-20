import { twoActorTest } from "../../../lib/actor_test";
import { AssetKind } from "../../../lib/asset";

setTimeout(function() {
    twoActorTest("rfc003-btc-eth-alice-underfunds-bob-aborts", async function({
        alice,
        bob,
    }) {
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
    });

    twoActorTest("rfc003-btc-eth-bob-underfunds-both-refund", async function({
        alice,
        bob,
    }) {
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
    });

    run();
}, 0);
