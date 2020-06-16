import { oneActorTest, twoActorTest } from "../src/actor_test";
import "../src/schema_matcher";
import * as sirenJsonSchema from "../siren.schema.json";
import * as sirenRootJsonSchema from "../root.schema.json";
import { siren } from "comit-sdk";
import axios from "axios";
import { createDefaultSwapRequest } from "../src/utils";
import { Actor } from "../src/actors/actor";
import * as swapPropertiesJsonSchema from "../swap.schema.json";
import { Rfc003Actor } from "../src/actors/rfc003_actor";

// ******************************************** //
// Siren Schema tests                                 //
// ******************************************** //

async function assertValidSirenDocument(
    swapsEntity: siren.Entity,
    alice: Actor
) {
    const selfLink = swapsEntity.links.find((link: siren.Link) =>
        link.rel.includes("self")
    ).href;

    const swapResponse = await alice.cnd.fetch(selfLink);
    const swapEntity = swapResponse.data as siren.Entity;

    expect(swapEntity).toMatchSchema(sirenJsonSchema);
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
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
            // Alice send swap request to Bob
            await alice.actor.cnd.postSwap(await createDefaultSwapRequest(bob));

            const aliceSwapEntity = await alice
                .pollCndUntil("/swaps", (body) => body.entities.length > 0)
                .then(
                    (body) =>
                        body
                            .entities[0] as siren.EmbeddedRepresentationSubEntity
                );

            await assertValidSirenDocument(aliceSwapEntity, alice.actor);

            const bobsSwapEntity = await bob
                .pollCndUntil("/swaps", (body) => body.entities.length > 0)
                .then(
                    (body) =>
                        body
                            .entities[0] as siren.EmbeddedRepresentationSubEntity
                );
            await assertValidSirenDocument(bobsSwapEntity, bob.actor);
        })
    );
});
