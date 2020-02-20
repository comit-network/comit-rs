import { twoActorTest } from "../../../lib/actor_test";
import { AssetKind } from "../../../lib/asset";

setTimeout(function() {
    twoActorTest("rfc003-btc-eth-alice-redeems-bob-redeems", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
        await bob.accept();

        await alice.fund();
        await bob.fund();

        await alice.redeem();
        await bob.redeem();

        await alice.assertSwapped();
        await bob.assertSwapped();
    });

    run();
}, 0);
