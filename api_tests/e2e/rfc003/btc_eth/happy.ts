import { createActors } from "../../../lib_sdk/create_actors";

setTimeout(function() {
    describe("happy path", function() {
        this.timeout(20000);
        it("bitcoin ether", async function() {
            const { alice, bob } = await createActors(
                "e2e-rfc003-btc-eth-happy.log"
            );

            await alice.sendRequest("bitcoin", "ether");
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
