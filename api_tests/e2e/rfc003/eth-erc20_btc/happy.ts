import { twoActorTest } from "../../../lib/actor_test";
import { AssetKind } from "../../../lib/asset";

setTimeout(function() {
    twoActorTest(
        "rfc003-eth-erc20_btc-alice-redeems-bob-redeems",
        async function({ alice, bob }) {
            await alice.sendRequest(AssetKind.Erc20, AssetKind.Bitcoin);
            await bob.accept();

            await alice.deploy();
            await alice.fund();
            await bob.fund();

            await alice.redeem();
            await bob.redeem();

            await alice.assertSwapped();
            await bob.assertSwapped();
        }
    );

    run();
}, 0);
