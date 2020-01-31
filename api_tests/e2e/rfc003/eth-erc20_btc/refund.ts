import { twoActorTest } from "../../../lib_sdk/actor_test";
import { AssetKind } from "../../../lib_sdk/asset";

setTimeout(function() {
    twoActorTest(
        "rfc003-eth-erc20_btc-bob-refunds-alice-refunds",
        async function({ alice, bob }) {
            await alice.sendRequest(AssetKind.Erc20, AssetKind.Bitcoin);
            await bob.accept();

            await alice.deploy();
            await alice.fund();
            await bob.fund();

            await alice.refund();
            await bob.refund();

            await alice.assertRefunded();
            await bob.assertRefunded();
        }
    );

    run();
}, 0);
