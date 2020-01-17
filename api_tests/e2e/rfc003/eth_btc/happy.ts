import { twoActorTest } from "../../../lib_sdk/actor_test";
import { AssetKind } from "../../../lib_sdk/asset";

setTimeout(function() {
    twoActorTest("rfc003-eth-btc-alice-redeems-bob-redeems", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
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
