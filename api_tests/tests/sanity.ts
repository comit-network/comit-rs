/**
 * @cndConfigOverride ethereum.chain_id = 1337
 * @cndConfigOverride ethereum.tokens.dai = 0x0000000000000000000000000000000000000000
 */

import {
    startAlice,
    startAliceAndBob,
    createAliceAndBob,
} from "../src/actor_test";
import SwapFactory from "../src/swap_factory";
import { merge } from "lodash";
import { HarnessGlobal } from "../src/environment";

declare var global: HarnessGlobal;

// ******************************************** //
// Sanity tests                                 //
// ******************************************** //

describe("Sanity", () => {
    it(
        "invalid-swap-yields-404",
        startAlice(async (alice) => {
            const promise = alice.cnd.fetch(
                "/swaps/deadbeef-dead-beef-dead-deadbeefdead"
            );

            await expect(promise).rejects.toMatchObject({
                status: 404,
                title: "Swap not found.",
            });
        })
    );

    it(
        "returns-invalid-body-for-bad-json-herc20-halbit",
        startAlice(async (alice) => {
            const promise = alice.cnd.createHerc20Halbit({
                // @ts-ignore
                garbage: true,
            });

            await expect(promise).rejects.toMatchObject({
                status: 400,
                title: "Invalid body.",
            });
        })
    );

    it(
        "returns-invalid-body-for-bad-json-halbit-herc20",
        startAlice(async (alice) => {
            const promise = alice.cnd.createHalbitHerc20({
                // @ts-ignore
                garbage: true,
            });

            await expect(promise).rejects.toMatchObject({
                status: 400,
                title: "Invalid body.",
            });
        })
    );

    it(
        "returns-invalid-body-for-bad-json-herc20-hbit",
        startAlice(async (alice) => {
            const promise = alice.cnd.createHerc20Hbit({
                // @ts-ignore
                garbage: true,
            });

            await expect(promise).rejects.toMatchObject({
                status: 400,
                title: "Invalid body.",
            });
        })
    );

    it(
        "returns-invalid-body-for-bad-json-hbit-herc20",
        startAlice(async (alice) => {
            const promise = alice.cnd.createHbitHerc20({
                // @ts-ignore
                garbage: true,
            });

            await expect(promise).rejects.toMatchObject({
                status: 400,
                title: "Invalid body.",
            });
        })
    );

    it(
        "alice-has-empty-peer-list",
        startAlice(async (alice) => {
            const promise = alice.cnd.fetch("/peers");

            await expect(promise).resolves.toMatchObject({
                status: 200,
                data: { peers: [] },
            });
        })
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
        })
    );

    it(
        "create-herc20-halbit-returns-bad-request-when-no-lnd-node",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).herc20Halbit;

            const expectedProblem = {
                status: 400,
                title: "lightning is not configured.",
                detail:
                    "lightning ledger is not properly configured, swap involving this ledger are not available.",
            };

            await expect(
                alice.cnd.createHerc20Halbit(bodies.alice)
            ).rejects.toMatchObject(expectedProblem);
            await expect(
                bob.cnd.createHerc20Halbit(bodies.bob)
            ).rejects.toMatchObject(expectedProblem);
        })
    );

    it(
        "create-halbit-herc20-returns-bad-request-when-no-lnd-node",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).halbitHerc20;

            const expectedProblem = {
                status: 400,
                title: "lightning is not configured.",
                detail:
                    "lightning ledger is not properly configured, swap involving this ledger are not available.",
            };

            await expect(
                alice.cnd.createHalbitHerc20(bodies.alice)
            ).rejects.toMatchObject(expectedProblem);
            await expect(
                bob.cnd.createHalbitHerc20(bodies.bob)
            ).rejects.toMatchObject(expectedProblem);
        })
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

            const currentConfig = bob.cndInstance.getConfigFile();
            const updatedConfig = merge(currentConfig, configOverride);

            bob.cndInstance.setConfigFile(updatedConfig);
            await bob.cndInstance.start();

            const aliceId = await alice.cnd.getPeerId();
            await bob.pollUntilConnectedTo(aliceId);
        })
    );
});
