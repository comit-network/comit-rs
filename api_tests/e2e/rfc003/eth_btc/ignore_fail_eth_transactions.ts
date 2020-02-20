import { twoActorTest } from "../../../lib/actor_test";
import { AssetKind } from "../../../lib/asset";

setTimeout(function() {
    twoActorTest("rfc003-eth-btc-alpha-deploy-fails", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
        await bob.accept();

        await alice.fundLowGas("0x1b000");

        await alice.assertAlphaNotDeployed();
        await bob.assertAlphaNotDeployed();
        await bob.assertBetaNotDeployed();
        await alice.assertBetaNotDeployed();
    });

    run();
}, 0);
