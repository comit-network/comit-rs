/**
 * @ledger bitcoin
 * @ledger ethereum
 * @fakeTreasuryService true
 */
import { startConnectedCndAndNectar } from "../src/actor_test";
import { MarketEntity, Position } from "../src/cnd_client/payload";
import { sleep } from "../src/utils";

test(
    "given_cnd_and_nectar_when_cnd_publishes_a_matching_buy_order_then_successful_swap",
    startConnectedCndAndNectar(async ({ alice, bob }) => {
        await bob.saveBalancesBeforeSwap();
        await alice.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length > 0
        );

        await alice.makeBtcDaiOrder(Position.Buy, "0.1", "9450"); // This matches what nectar publishes.
        await alice.waitForSwap();

        await bob.stop();

        await alice.assertAndExecuteNextAction("deploy");
        await alice.assertAndExecuteNextAction("fund");

        await bob.start();

        await alice.assertAndExecuteNextAction("redeem");

        await alice.stop();
        await sleep(2000);
        await alice.start();

        await alice.waitUntilSwapped();

        await alice.assertBalancesAfterSwap();
        await alice.assertOrderClosed();
        await alice.assertSwapInactive();
        await bob.assertBalancesChangedBy({
            bitcoin: -(10_000_000n + 7650n), // nectar pays order quantity + the funding fee
            dai: 945_000_000_000_000_000_000n, // = 0.1 * 9450 * 10^18
        });
    })
);

test(
    "given_cnd_and_nectar_when_cnd_publishes_a_matching_sell_order_then_successful_swap",
    startConnectedCndAndNectar(async ({ alice, bob }) => {
        await bob.saveBalancesBeforeSwap();
        await alice.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length > 0
        );

        await alice.makeBtcDaiOrder(Position.Sell, "0.1", "8550"); // This matches what nectar publishes.
        await alice.waitForSwap();

        await bob.stop();

        await alice.assertAndExecuteNextAction("fund");

        await alice.stop();
        await bob.start();

        await sleep(2000);
        await alice.start();

        await alice.assertAndExecuteNextAction("redeem");

        await alice.waitUntilSwapped();

        await alice.assertBalancesAfterSwap();
        await alice.assertOrderClosed();
        await alice.assertSwapInactive();
        await bob.assertBalancesChangedBy({
            bitcoin: 10000000n - 16200n, // nectar receives order quantity but pays the redeem fee
            dai: -855_000_000_000_000_000_000n, // = 0.1 * 8550 * 10^18
        });
    })
);
