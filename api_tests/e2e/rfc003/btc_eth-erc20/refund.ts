import { twoActorTest } from "../../../lib_sdk/actor_test";
import { AssetKind } from "../../../lib_sdk/asset";

setTimeout(function() {
    twoActorTest(
        "rfc003-btc-eth-erc20-bob-refunds-alice-refunds",
        async function({ alice, bob }) {
            await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Erc20);
            await bob.accept();

            await alice.fund();
            await bob.deploy();
            await bob.fund();

            await alice.refund();
            await bob.refund();

            await alice.assertRefunded();
            await bob.assertRefunded();
        }
    );

    run();
}, 0);
