/**
 * @logDir rfc003
 */

import { EmbeddedRepresentationSubEntity, Entity, Link } from "../../gen/siren";
import { Actor } from "../../lib/actors/actor";
import { expect, request } from "chai";
import "chai/register-should";
import "../../lib/setup_chai";
import * as sirenJsonSchema from "../../siren.schema.json";
import * as swapPropertiesJsonSchema from "../../swap.schema.json";
import { twoActorTest } from "../../lib/actor_test";
import { createDefaultSwapRequest, DEFAULT_ALPHA } from "../../lib/utils";
import { Action } from "comit-sdk";

// ******************************************** //
// RFC003 schema tests                          //
// ******************************************** //

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

describe("Rfc003 schema tests", () => {
    it("get-all-swaps-is-valid-siren", async function() {
        await twoActorTest("get-all-swaps-is-valid-siren", async function({
            alice,
        }) {
            const res = await request(alice.cndHttpApiUrl()).get("/swaps");

            expect(res.body).to.be.jsonSchema(sirenJsonSchema);
        });
    });
    it("get-single-swap-is-valid-siren", async function() {
        await twoActorTest("get-single-swap-is-valid-siren", async function({
            alice,
            bob,
        }) {
            // Alice send swap request to Bob
            await alice.cnd.postSwap(await createDefaultSwapRequest(bob));

            const aliceSwapEntity = await alice
                .pollCndUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            await assertValidSirenDocument(
                aliceSwapEntity,
                alice,
                "[Alice] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema"
            );

            const bobsSwapEntity = await bob
                .pollCndUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );
            await assertValidSirenDocument(
                bobsSwapEntity,
                bob,
                "[Bob] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema"
            );
        });
    });

    it("get-single-swap-contains-link-to-rfc", async function() {
        await twoActorTest(
            "get-single-swap-contains-link-to-rfc",
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
});

// ******************************************** //
// RFC003 Swap Reject                           //
// ******************************************** //

async function assertSwapsInProgress(actor: Actor, message: string) {
    const res = await request(actor.cndHttpApiUrl()).get("/swaps");

    const swapEntities = res.body.entities as EmbeddedRepresentationSubEntity[];

    expect(swapEntities.map(entity => entity.properties, message))
        .to.each.have.property("status")
        .that.is.equal("IN_PROGRESS");
}

describe("Rfc003 schema swap reject tests", () => {
    it("alice-can-make-default-swap-request", async function() {
        await twoActorTest(
            "alice-can-make-default-swap-request",
            async function({ alice, bob }) {
                // Alice should be able to send two swap requests to Bob
                const url1 = await alice.cnd.postSwap({
                    ...(await createDefaultSwapRequest(bob)),
                    alpha_asset: {
                        name: DEFAULT_ALPHA.asset.name,
                        quantity: DEFAULT_ALPHA.asset.quantity.reasonable,
                    },
                });

                const url2 = await alice.cnd.postSwap({
                    ...(await createDefaultSwapRequest(bob)),
                    alpha_asset: {
                        name: DEFAULT_ALPHA.asset.name,
                        quantity: DEFAULT_ALPHA.asset.quantity.stingy,
                    },
                });

                await assertSwapsInProgress(
                    alice,
                    "[Alice] Shows the swaps as IN_PROGRESS in GET /swaps"
                );

                // make sure bob processed the swaps fully
                await bob.pollSwapDetails(url1);
                await bob.pollSwapDetails(url2);

                await assertSwapsInProgress(
                    bob,
                    "[Bob] Shows the swaps as IN_PROGRESS in /swaps"
                );
            }
        );
    });

    it("bob-can-decline-swap", async function() {
        await twoActorTest("bob-can-decline-swap", async function({
            alice,
            bob,
        }) {
            // Alice should be able to send two swap requests to Bob
            const aliceReasonableSwap = await alice.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.reasonable,
                },
            });

            const aliceStingySwap = await alice.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.stingy,
                },
            });

            const bobSwapDetails = await bob.pollSwapDetails(aliceStingySwap);

            expect(
                bobSwapDetails.properties,
                "[Bob] Has the RFC-003 parameters when GETing the swap"
            ).jsonSchema(swapPropertiesJsonSchema);
            expect(
                bobSwapDetails.actions,
                "[Bob] Has the accept and decline actions when GETing the swap"
            ).containSubset([
                {
                    name: "accept",
                },
                {
                    name: "decline",
                },
            ]);

            /// Decline the swap
            const decline = bobSwapDetails.actions.find(
                (action: Action) => action.name === "decline"
            );
            const declineRes = await bob.cnd.executeSirenAction(decline);

            declineRes.should.have.status(200);

            expect(
                await bob.pollCndUntil(
                    aliceStingySwap,
                    entity =>
                        entity.properties.state.communication.status ===
                        "DECLINED"
                ),
                "[Bob] Should be in the Declined State after declining a swap request providing a reason"
            ).to.exist;

            const aliceReasonableSwapDetails = await alice.pollSwapDetails(
                aliceReasonableSwap
            );
            const aliceStingySwapDetails = await alice.pollSwapDetails(
                aliceStingySwap
            );

            expect(
                aliceStingySwapDetails.properties.state.communication.status,
                "[Alice] Should be in the Declined State after Bob declines a swap"
            ).to.eq("DECLINED");

            expect(
                aliceReasonableSwapDetails.properties.state.communication
                    .status,
                "[Alice] Should be in the SENT State for the other swap request"
            ).to.eq("SENT");
        });
    });
});
