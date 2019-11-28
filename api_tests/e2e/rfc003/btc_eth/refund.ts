import { AssetKind } from "../../../lib_sdk/asset";
import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    describe("Bob can refund and then Alice refunds", function() {
        this.timeout(60000);
        it("bitcoin ether", async function() {
            const { alice, bob } = await createActors(
                "e2e-rfc003-btc-eth-refund.log"
            );

            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();

            await alice.fund();
            await bob.fund();

            await bob.refund();
            await alice.refund();

            await bob.assertRefunded();
            await alice.assertRefunded();
        });
    });
    run();
}, 0);
