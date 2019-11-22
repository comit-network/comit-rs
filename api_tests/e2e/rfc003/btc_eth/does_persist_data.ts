import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    describe("cnd", function() {
        this.timeout(60000);
        it("does persist data between restarts", async function() {
            const { alice, bob } = await createActors("does-persist-data.log");

            await alice.sendRequest();
            await bob.accept();

            await alice.restart();

            await alice.assertHasCurrentSwap();
        });
    });
    run();
}, 0);
