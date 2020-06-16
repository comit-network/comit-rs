/**
 * @ledger bitcoin
 * @ledger ethereum
 */

import { oneActorTest, twoActorTest } from "../src/actor_test";
import SwapFactory from "../src/actors/swap_factory";
import { sleep } from "../src/utils";
import "../src/schema_matcher";
import * as sirenRootJsonSchema from "../root.schema.json";
import * as sirenSwapJsonSchema from "../swap.schema.json";
import { siren } from "comit-sdk";
import axios from "axios";
import { Actor } from "../src/actors/actor";
import * as swapPropertiesJsonSchema from "../swap.schema.json";
import { SwapResponse } from "../src/payload";

// ******************************************** //
// Siren Schema tests                                 //
// ******************************************** //

async function assertValidSirenSwapDocument(
    swapsEntity: siren.Entity,
    alice: Actor
) {
    const selfLink = swapsEntity.links.find((link: siren.Link) =>
        link.rel.includes("self")
    ).href;

    const swapResponse = await alice.cnd.fetch(selfLink);
    const swapEntity = swapResponse.data as siren.Entity;

    expect(swapEntity).toMatchSchema(sirenSwapJsonSchema);
    expect(swapEntity.properties).toMatchSchema(swapPropertiesJsonSchema);
}

describe("Siren Schema", () => {
    it(
        "can-fetch-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await alice.cnd.fetch("/");

            expect(res.status).toBe(200);
            expect(res.data).toMatchSchema(sirenRootJsonSchema);
        })
    );

    it(
        "returns-listen-addresses-on-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await axios({
                baseURL: alice.cndHttpApiUrl(),
                url: "/",
                headers: { accept: "application/vnd.siren+json" },
            });
            const body = res.data as any;

            expect(typeof body.properties.id).toBe("string");
            expect(
                Array.isArray(body.properties.listen_addresses)
            ).toBeTruthy();
            // At least 2 ipv4 addresses, lookup and external interface
            expect(
                body.properties.listen_addresses.length
            ).toBeGreaterThanOrEqual(2);
        })
    );

    it(
        "returns-links-to-create-swap-endpoints-on-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await axios({
                baseURL: alice.cndHttpApiUrl(),
                url: "/",
                headers: { accept: "application/vnd.siren+json" },
            });
            const body = res.data as any;
            const links = body.links;

            const swapsLink = links.find(
                (link: siren.Link) =>
                    link.rel.length === 1 &&
                    link.rel.includes("collection") &&
                    link.class.length === 1 &&
                    link.class.includes("swaps")
            );

            expect(swapsLink).toMatchObject({
                rel: ["collection"],
                class: ["swaps"],
                href: "/swaps",
            });
        })
    );

    it(
        "get-single-swap-is-valid-siren",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (
                await SwapFactory.newSwap(alice, bob, {
                    ledgers: {
                        alpha: "bitcoin",
                        beta: "ethereum",
                    },
                })
            ).hbitHerc20;

            await alice.createHbitHerc20Swap(bodies.alice);
            await bob.createHbitHerc20Swap(bodies.bob);

            // Wait for the announce protocol to complete.
            await sleep(2000);

            const responseAlice = await alice.cnd.fetch<SwapResponse>(
                alice.swap.self
            );
            expect(responseAlice.status).toEqual(200);
            const entityAlice = responseAlice.data;
            await assertValidSirenSwapDocument(entityAlice, alice);

            const responseBob = await bob.cnd.fetch<SwapResponse>(
                bob.swap.self
            );
            expect(responseBob.status).toEqual(200);
            const entityBob = responseBob.data;
            await assertValidSirenSwapDocument(entityBob, bob);
        })
    );
});
