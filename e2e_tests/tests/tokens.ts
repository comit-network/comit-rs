/**
 * @cndConfigOverride ethereum.chain_id = 1337
 * @cndConfigOverride ethereum.tokens.dai = 0x0000000000000000000000000000000000000000
 */

import { startAlice } from "../src/actor_test";

test(
    "given_a_config_when_listing_tokens_then_should_return_token_from_config",
    startAlice(async (alice) => {
        const tokens = await alice.cnd.fetch("/tokens").then((r) => r.data);

        expect(tokens).toContainEqual({
            symbol: "DAI",
            address: "0x0000000000000000000000000000000000000000",
            decimals: 18,
        });
    })
);
