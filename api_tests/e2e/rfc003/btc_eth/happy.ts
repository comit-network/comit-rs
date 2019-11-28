import { AssetKind } from "../../../lib_sdk/asset";
import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    describe("happy path", function() {
        this.timeout(60000);
        it("bitcoin ether", async function() {
            const { alice, bob } = await createActors(
                "e2e-rfc003-btc-eth-happy.log"
            );

            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await alice.assertAlphaFunded();
            await bob.assertAlphaFunded();

            await bob.fund();
            await alice.assertBetaFunded();
            await bob.assertBetaFunded();

            await alice.redeem();
            await bob.redeem();

            await alice.assertBetaRedeemed();
            await alice.assertAlphaRedeemed();
            await bob.assertBetaRedeemed();
            await bob.assertAlphaRedeemed();

            await alice.assertSwapped();
            await bob.assertSwapped();
        });
    });
    run();
}, 0);
