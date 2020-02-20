import { twoActorTest } from "../../../lib/actor_test";
import { AssetKind } from "../../../lib/asset";

setTimeout(function() {
    twoActorTest("rfc003-btc-eth-bob-refunds-alice-refunds", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
        await bob.accept();

        await alice.fund();
        await bob.fund();

        await bob.refund();
        await alice.refund();

        await bob.assertRefunded();
        await alice.assertRefunded();
    });

    twoActorTest("rfc003-btc-eth-alice-refunds-bob-refunds", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
        await bob.accept();

        await alice.fund();
        await bob.fund();

        await alice.refund();
        await bob.refund();

        await alice.assertRefunded();
        await bob.assertRefunded();
    });

    run();
}, 0);
