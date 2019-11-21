import { sleep } from "../../../lib/util";
import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    describe("cnd", function() {
        this.timeout(60000);
        it("persists data between restarts", async function() {
            const { alice, bob } = await createActors("persist-data.log");

            await alice.sendRequest();
            await bob.accept();

            await sleep(1000); // Give accept message time to get to Alice before restarting.

            await alice.restart();

            await alice.assertHasNumSwaps(1);
        });
    });
    run();
}, 0);
