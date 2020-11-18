/**
 * @cndConfigOverride ethereum.chain_id = 1337
 * @cndConfigOverride ethereum.tokens.dai = 0x0000000000000000000000000000000000000000
 */

import { createAliceAndBob, startAlice } from "../src/actor_test";
import { merge } from "lodash";

// ******************************************** //
// Sanity tests                                 //
// ******************************************** //

describe("Sanity", () => {
    it(
        "invalid-swap-yields-404",
        startAlice(async (alice) => {
            const promise = alice.cnd.fetch(
                "/swaps/deadbeef-dead-beef-dead-deadbeefdead",
            );

            await expect(promise).rejects.toMatchObject({
                status: 404,
                title: "Swap not found.",
            });
        }),
    );

    it(
        "alice-has-empty-peer-list",
        startAlice(async (alice) => {
            const promise = alice.cnd.fetch("/peers");

            await expect(promise).resolves.toMatchObject({
                status: 200,
                data: { peers: [] },
            });
        }),
    );

    it(
        "returns-listen-addresses-on-root-document",
        startAlice(async (alice) => {
            const res = await alice.cnd.fetch("/");

            const body = res.data as { id: string; listen_addresses: string[] };

            expect(typeof body.id).toEqual("string");
            expect(body.id).toBeTruthy();
            // At least 2 ipv4 addresses, lookup and external interface
            expect(body.listen_addresses.length).toBeGreaterThanOrEqual(2);
        }),
    );

    it(
        "bob-connects-to-alice-using-config",
        createAliceAndBob(async ([alice, bob]) => {
            await alice.cndInstance.start();

            const aliceAddresses = await alice.cnd.getPeerListenAddresses();
            const configOverride = {
                network: {
                    peer_addresses: aliceAddresses,
                },
            };

            const currentConfig = bob.cndInstance.config;
            const updatedConfig = merge(currentConfig, configOverride);

            bob.cndInstance.config = updatedConfig;
            await bob.cndInstance.start();

            const aliceId = await alice.cnd.getPeerId();
            await bob.pollUntilConnectedTo(aliceId);
        }),
    );
});
