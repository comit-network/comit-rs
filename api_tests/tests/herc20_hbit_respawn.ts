/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { startConnectedAliceAndBob } from "../src/actor_test";
import { sleep } from "../src/utils";
import { Position } from "../src/cnd_client/payload";

describe("herc20-hbit-respawn", () => {
    it(
        "herc20-hbit-alice-misses-bob-funds",
        startConnectedAliceAndBob(async ([alice, bob]) => {
            await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
            await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);

            await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

            await alice.assertAndExecuteNextAction("deploy");
            await alice.assertAndExecuteNextAction("fund");

            await alice.stop();

            // Action happens while alice is down
            await bob.assertAndExecuteNextAction("fund");
            await sleep(2000);

            await alice.start();

            await alice.assertAndExecuteNextAction("redeem");
            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );

    it(
        "herc20-hbit-bob-misses-alice-redeems",
        startConnectedAliceAndBob(async ([alice, bob]) => {
            await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
            await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);

            await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

            await alice.assertAndExecuteNextAction("deploy");
            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("fund");

            await bob.stop();

            // Action happens while bob is down
            await alice.assertAndExecuteNextAction("redeem");
            await sleep(2000);

            await bob.start();

            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );

    it(
        "hbit-herc20-alice-misses-bob-deploys-and-funds",
        startConnectedAliceAndBob(async ([alice, bob]) => {
            await alice.makeBtcDaiOrder(Position.Sell, 0.2, 9000);
            await bob.makeBtcDaiOrder(Position.Buy, 0.2, 9000);

            await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

            await alice.assertAndExecuteNextAction("fund");

            await alice.stop();

            // Actions happen while alice is down
            await bob.assertAndExecuteNextAction("deploy");
            await bob.assertAndExecuteNextAction("fund");
            await sleep(2000);

            await alice.start();

            await alice.assertAndExecuteNextAction("redeem");
            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );

    it(
        "hbit-herc20-bob-down-misses-alice-redeems",
        startConnectedAliceAndBob(async ([alice, bob]) => {
            await alice.makeBtcDaiOrder(Position.Sell, 0.2, 9000);
            await bob.makeBtcDaiOrder(Position.Buy, 0.2, 9000);

            await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("deploy");
            await bob.assertAndExecuteNextAction("fund");

            await bob.stop();

            // Action happens while bob is down
            await alice.assertAndExecuteNextAction("redeem");
            await sleep(2000);

            await bob.start();

            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );
});
