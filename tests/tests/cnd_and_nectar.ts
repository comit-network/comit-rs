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

        await alice.makeBtcDaiOrder(Position.Buy, "0.09990725", "9450"); // This matches what nectar publishes.
        await alice.waitForSwap();

        await alice.assertAndExecuteNextAction("deploy");
        await alice.assertAndExecuteNextAction("fund");
        await alice.assertAndExecuteNextAction("redeem");

        // Wait until the wallets sees the new balance.
        await sleep(2000);

        await alice.assertBalancesAfterSwap();
        await bob.assertBalancesChangedBy({
            bitcoin: -(9990725n + 3060n), // nectar pays order quantity + the funding fee
            dai: 944_123_512_500_000_000_000n, // = 0.09990725 * 9450 * 10^18
        });
    })
);
