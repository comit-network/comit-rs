// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
// These test do not use the sdk so that we can test edge cases
import { twoActorTest } from "../lib_sdk/actor_test";
import { expect, request } from "chai";
import "chai/register-should";
import "../lib_sdk/setup_chai";
import { Actor } from "../lib_sdk/actors/actor";
import { EmbeddedRepresentationSubEntity, Entity, Link } from "../gen/siren";
import * as sirenJsonSchema from "../siren.schema.json";
import * as swapPropertiesJsonSchema from "../swap.schema.json";
import { createDefaultSwapRequest } from "../lib_sdk/utils";

async function assertValidSirenDocument(
    swapsEntity: Entity,
    alice: Actor,
    message: string
) {
    const selfLink = swapsEntity.links.find((link: Link) =>
        link.rel.includes("self")
    ).href;

    const swapResponse = await request(alice.cndHttpApiUrl()).get(selfLink);
    const swapEntity = swapResponse.body as Entity;

    expect(swapEntity, message).to.be.jsonSchema(sirenJsonSchema);
    expect(swapEntity.properties, message).to.be.jsonSchema(
        swapPropertiesJsonSchema
    );
}

setTimeout(async function() {
    describe("Response shape", () => {
        twoActorTest(
            "[Alice] Response for GET /swaps is a valid siren document",
            async function({ alice }) {
                const res = await request(alice.cndHttpApiUrl()).get("/swaps");

                expect(res.body).to.be.jsonSchema(sirenJsonSchema);
            }
        );

        twoActorTest(
            "Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema",
            async function({ alice, bob }) {
                // Alice send swap request to Bob
                await alice.cnd.postSwap(await createDefaultSwapRequest(bob));

                const aliceSwapEntity = await alice
                    .pollCndUntil("/swaps", body => body.entities.length > 0)
                    .then(
                        body =>
                            body.entities[0] as EmbeddedRepresentationSubEntity
                    );

                await assertValidSirenDocument(
                    aliceSwapEntity,
                    alice,
                    "[Alice] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema"
                );

                const bobsSwapEntity = await bob
                    .pollCndUntil("/swaps", body => body.entities.length > 0)
                    .then(
                        body =>
                            body.entities[0] as EmbeddedRepresentationSubEntity
                    );
                await assertValidSirenDocument(
                    bobsSwapEntity,
                    bob,
                    "[Bob] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema"
                );
            }
        );

        twoActorTest(
            "[Alice] Response for GET /swaps/rfc003/{} contains a link to the protocol spec",
            async function({ alice, bob }) {
                // Alice send swap request to Bob
                await alice.cnd.postSwap(await createDefaultSwapRequest(bob));

                const aliceSwapEntity = await alice
                    .pollCndUntil("/swaps", body => body.entities.length > 0)
                    .then(
                        body =>
                            body.entities[0] as EmbeddedRepresentationSubEntity
                    );

                const protocolLink = aliceSwapEntity.links.find((link: Link) =>
                    link.rel.includes("describedBy")
                );

                expect(protocolLink).to.be.deep.equal({
                    rel: ["describedBy"],
                    class: ["protocol-spec"],
                    type: "text/html",
                    href:
                        "https://github.com/comit-network/RFCs/blob/master/RFC-003-SWAP-Basic.adoc",
                });
            }
        );
    });

    run();
}, 0);
