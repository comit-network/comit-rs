import { Actor } from "../../../lib_sdk/actors/actor";
import { AssetKind } from "../../../lib_sdk/asset";
import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    let alice: Actor;
    let bob: Actor;

    beforeEach(async function() {
        this.timeout(20000);
        const actors = await createActors(
            "e2e-rfc003-btc-eth-inverted-refund.log"
        );
        alice = actors.alice;
        bob = actors.bob;
    });

    afterEach(() => {
        if (alice) {
            alice.stop();
        }
        if (bob) {
            bob.stop();
        }
    });

    describe("Alice refunds then Bob refunds", function() {
        this.timeout(60000);
        it("bitcoin ether", async function() {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();
            await alice.fund();
            await bob.fund();
            await alice.refund();
            await bob.refund();
            await alice.assertRefunded();
            await bob.assertRefunded();
        });
    });
    run();
}, 0);
