import { twoActorTest } from "../../../lib/actor_test";
import { AssetKind } from "../../../lib/asset";

setTimeout(function() {
    twoActorTest("rfc003-eth-btc-bob-refunds-alice-refunds", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
        await bob.accept();

        await alice.fund();
        await bob.fund();

        await bob.refund();
        await alice.refund();

        await bob.assertRefunded();
        await alice.assertRefunded();
    });

    run();
}, 0);
