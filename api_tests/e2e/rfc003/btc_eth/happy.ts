import { Actor } from "../../../lib_sdk/actors/actor";
import { AssetKind } from "../../../lib_sdk/asset";
import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    let alice: Actor;
    let bob: Actor;

    beforeEach(async function() {
        this.timeout(20000);
        const actors = await createActors("e2e-rfc003-btc-eth-happy.log");
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

    describe("happy path", function() {
        this.timeout(60000);
        it("bitcoin ether", async function() {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
            await bob.accept();
            await alice.fund();
            await bob.fund();
            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        });
    });
    run();
}, 0);
