import { twoActorTest } from "../../../lib_sdk/actor_test";

setTimeout(function() {
    twoActorTest("restart-cnd-after-swap-is-accepted", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest();
        await bob.accept();

        await alice.currentSwapIsAccepted();
        await bob.currentSwapIsAccepted();

        await alice.restart();
        await bob.restart();

        await alice.currentSwapIsAccepted();
        await bob.currentSwapIsAccepted();
    });

    run();
}, 0);
