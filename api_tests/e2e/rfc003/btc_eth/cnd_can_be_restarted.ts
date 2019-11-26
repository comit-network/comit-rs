import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    describe("cnd can be restarted", function() {
        this.timeout(60000);
        it("after the swap was accepted", async function() {
            const { alice, bob } = await createActors(
                "cnd_can_be_restarted.log"
            );

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
