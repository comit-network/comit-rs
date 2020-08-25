/**
 * @cndConfigOverride ethereum.chain_id = 1337
 * @cndConfigOverride ethereum.tokens.dai = 0x0000000000000000000000000000000000000000
 */
import { twoActorTest } from "../src/actor_test";
import { MarketEntity, Position } from "../src/payload";

test(
    "given_two_connected_nodes_when_other_node_publishes_order_then_it_is_returned_in_the_market",
    twoActorTest(async ({ alice, bob }) => {
        await alice.connect(bob);
        await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);
        const bobsPeerId = await bob.cnd.getPeerId();

        const aliceMarket = await alice.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length > 0
        );
        const bobMarket = await bob.pollCndUntil<MarketEntity>(
            "/markets/BTC-DAI",
            (market) => market.entities.length > 0
        );

        const expectedOrder = {
            position: Position.Sell,
            maker: bobsPeerId,
            quantity: {
                currency: "BTC",
                value: "20000000",
                decimals: 8,
            },
            price: {
                currency: "DAI",
                value: "9000000000000000000000",
                decimals: 18,
            },
        };
        expect(aliceMarket.entities[0].properties).toMatchObject({
            ...expectedOrder,
            ours: false,
        });
        expect(bobMarket.entities[0].properties).toMatchObject({
            ...expectedOrder,
            ours: true, // Bob should see his own order in the market
        });
    })
);
