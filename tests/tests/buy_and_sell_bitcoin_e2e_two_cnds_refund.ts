/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { startConnectedAliceAndBob } from "../src/actor_test";
import { sleep } from "../src/utils";
import { Position } from "../src/cnd_client/payload";

it(
    "alice-buys-bob-sells-both-refund",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Buy, "0.2", "9000");
        await bob.makeBtcDaiOrder(Position.Sell, "0.2", "9000");

        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        await alice.assertAndExecuteNextAction("deploy");
        await alice.assertAndExecuteNextAction("fund");

        await bob.assertAndExecuteNextAction("fund");

        await bob.waitForRefund();

        await bob.assertAndExecuteNextAction("refund");
        await alice.assertAndExecuteNextAction("refund");

        // Wait until the wallet sees the new balance.
        await sleep(2000);

        await alice.assertBalancesAfterRefund();
        await bob.assertBalancesAfterRefund();
    })
);

it(
    "alice-sells-bob-buys-both-refund",
    startConnectedAliceAndBob(async ([alice, bob]) => {
        await alice.makeBtcDaiOrder(Position.Sell, "0.2", "9000");
        await bob.makeBtcDaiOrder(Position.Buy, "0.2", "9000");

        await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

        await alice.assertAndExecuteNextAction("fund");

        await bob.assertAndExecuteNextAction("deploy");
        await bob.assertAndExecuteNextAction("fund");

        await bob.waitForRefund();

        await bob.assertAndExecuteNextAction("refund");
        await alice.assertAndExecuteNextAction("refund");

        // Wait until the wallet sees the new balance.
        await sleep(2000);

        await alice.assertBalancesAfterRefund();
        await bob.assertBalancesAfterRefund();
    })
);
