/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { startConnectedAliceAndBob } from "../src/actor_test";
import { Position } from "../src/cnd_client/payload";

it(
    "alice-buys-bob-sells-bitcoin",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");
        await bob.makeBtcDaiOrder(Position.Sell, "0.2", "9000");

        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        await alice.assertAndExecuteNextAction("deploy");
        await alice.assertAndExecuteNextAction("fund");

        await bob.assertAndExecuteNextAction("fund");

        await alice.assertAndExecuteNextAction("redeem");
        await bob.assertAndExecuteNextAction("redeem");

        await Promise.all([alice.waitUntilSwapped(), bob.waitUntilSwapped()]);

        await alice.assertBalancesAfterSwap();
        await alice.assertOrderClosed();
        await alice.assertSwapInactive();
        await bob.assertBalancesAfterSwap();
        await bob.assertOrderClosed();
        await bob.assertSwapInactive();
    }),
);

it(
    "alice-sells-bob-buys-bitcoin",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Sell, "0.2", "9000");
        await bob.makeBtcDaiOrder(Position.Buy, "0.2", "9000");

        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        await alice.assertAndExecuteNextAction("fund");

        await bob.assertAndExecuteNextAction("deploy");
        await bob.assertAndExecuteNextAction("fund");

        await alice.assertAndExecuteNextAction("redeem");
        await bob.assertAndExecuteNextAction("redeem");

        await Promise.all([alice.waitUntilSwapped(), bob.waitUntilSwapped()]);

        await alice.assertBalancesAfterSwap();
        await alice.assertOrderClosed();
        await alice.assertSwapInactive();
        await bob.assertBalancesAfterSwap();
        await bob.assertOrderClosed();
        await bob.assertSwapInactive();
    }),
);
