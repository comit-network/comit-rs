import { twoActorTest } from "../../../lib_sdk/actor_test";
import { AssetKind } from "../../../lib_sdk/asset";
import { parseEther } from "ethers/utils";

setTimeout(function() {
    twoActorTest("rfc003-btc-eth-alice-overfunds-bob-aborts", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(
            AssetKind.Bitcoin,
            AssetKind.Ether,
            "100000000",
            parseEther("10").toString()
        );
        await bob.accept();

        await alice.fundWithQuantity("150000000");

        await bob.assertAlphaIncorrectlyFunded();
        await bob.assertBetaNotDeployed();
        await alice.assertAlphaIncorrectlyFunded();
        await alice.assertBetaNotDeployed();

        await alice.refund();
        await alice.assertRefunded();

        await bob.assertBetaNotDeployed();
    });

    run();
}, 0);
