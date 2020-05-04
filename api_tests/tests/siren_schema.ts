import { oneActorTest, twoActorTest } from "../src/actor_test";
import "../src/schema_matcher";
import * as sirenJsonSchema from "../siren.schema.json";
import { EmbeddedRepresentationSubEntity, Entity, Link } from "comit-sdk";
import axios from "axios";
import { createDefaultSwapRequest } from "../src/utils";
import { Actor } from "../src/actors/actor";
import * as swapPropertiesJsonSchema from "../swap.schema.json";

// ******************************************** //
// Siren Schema tests                                 //
// ******************************************** //

async function assertValidSirenDocument(swapsEntity: Entity, alice: Actor) {
    const selfLink = swapsEntity.links.find((link: Link) =>
        link.rel.includes("self")
    ).href;

    const swapResponse = await alice.cnd.fetch(selfLink);
    const swapEntity = swapResponse.data as Entity;

    expect(swapEntity).toMatchSchema(sirenJsonSchema);
    expect(swapEntity.properties).toMatchSchema(swapPropertiesJsonSchema);
}

describe("Siren Schema", () => {
    it(
        "can-fetch-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await alice.cnd.fetch("/");

            expect(res.status).toBe(200);
            expect(res.data).toMatchSchema(sirenJsonSchema);
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
                (link: Link) =>
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

            const rfc003SwapsLink = links.find(
                (link: Link) =>
                    link.rel.length === 2 &&
                    link.rel.includes("collection") &&
                    link.rel.includes("edit") &&
                    link.class.length === 2 &&
                    link.class.includes("swaps") &&
                    link.class.includes("rfc003")
            );

            expect(rfc003SwapsLink).toMatchObject({
                rel: ["collection", "edit"],
                class: ["swaps", "rfc003"],
                href: "/swaps/rfc003",
            });
        })
    );

    it(
        "get-single-swap-is-valid-siren",
        twoActorTest(async ({ alice, bob }) => {
            // Alice send swap request to Bob
            await alice.cnd.postSwap(await createDefaultSwapRequest(bob));

            const aliceSwapEntity = await alice
                .pollCndUntil("/swaps", (body) => body.entities.length > 0)
                .then(
                    (body) =>
                        body.entities[0] as EmbeddedRepresentationSubEntity
                );

            await assertValidSirenDocument(aliceSwapEntity, alice);

            const bobsSwapEntity = await bob
                .pollCndUntil("/swaps", (body) => body.entities.length > 0)
                .then(
                    (body) =>
                        body.entities[0] as EmbeddedRepresentationSubEntity
                );
            await assertValidSirenDocument(bobsSwapEntity, bob);
        })
    );
});
