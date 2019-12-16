import { Actor } from "../../../lib_sdk/actors/actor";
import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    let alice: Actor;
    let bob: Actor;

    beforeEach(async function() {
        this.timeout(20000);
        const actors = await createActors("cnd_can_be_restarted.log");
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

    describe("cnd can be restarted", function() {
        this.timeout(60000);
        it("after the swap was accepted", async function() {
            await alice.sendRequest();
            await bob.accept();

            await alice.currentSwapIsAccepted();
            await bob.currentSwapIsAccepted();

            await alice.restart();
            await bob.restart();

            await alice.currentSwapIsAccepted();
            await bob.currentSwapIsAccepted();
        });
    });
    run();
}, 0);
