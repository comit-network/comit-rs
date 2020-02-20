import { twoActorTest } from "../../../lib/actor_test";
import { AssetKind } from "../../../lib/asset";
import { expect } from "chai";

setTimeout(function() {
    twoActorTest("rfc003-eth-btc-alice-redeems-with-high-fee", async function({
        alice,
        bob,
    }) {
        await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
        await bob.accept();

        await alice.fund();
        await bob.fund();

        const responsePromise = alice.redeemWithHighFee();

        await expect(responsePromise).to.be.rejected;
    });

    run();
}, 0);
