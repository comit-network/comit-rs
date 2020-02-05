// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
// These test do not use the sdk so that we can test edge cases
import { twoActorTest } from "../lib_sdk/actor_test";
import { expect, request } from "chai";
import "chai/register-should";
import "../lib/setup_chai";
import { EmbeddedRepresentationSubEntity } from "../gen/siren";
import * as swapPropertiesJsonSchema from "../swap.schema.json";
import { Actor } from "../lib_sdk/actors/actor";
import {createDefaultSwapRequest, DEFAULT_ALPHA} from "../lib_sdk/utils";

async function assertSwapsInProgress(actor: Actor, message: string) {
    const res = await request(actor.cndHttpApiUrl()).get("/swaps");

    const swapEntities = res.body.entities as EmbeddedRepresentationSubEntity[];

    expect(swapEntities.map(entity => entity.properties, message))
        .to.each.have.property("status")
        .that.is.equal("IN_PROGRESS");
}

setTimeout(async function() {
    describe("SWAP request DECLINED", () => {
        twoActorTest(
            "[Alice] Should be able to make first swap request via HTTP api",
            async function({ alice, bob }) {
                // setup

                // Alice should be able to send two swap requests to Bob
                await alice.cnd.postSwap({
                    ...(await createDefaultSwapRequest(bob)),
                    alpha_asset: {
                        name: DEFAULT_ALPHA.asset.name,
                        quantity: DEFAULT_ALPHA.asset.quantity.reasonable,
                    },
                });
                await alice.cnd.postSwap({
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
                await assertSwapsInProgress(
                    bob,
                    "[Bob] Shows the swaps as IN_PROGRESS in /swaps"
                );
            }
        );

        twoActorTest("[Bob] Decline one swap", async function({ alice, bob }) {
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

            const bobSwapDetails = await bob.pollSwapDetails(
                aliceStingySwap
            );

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
                action => action.name === "decline"
            );
            const declineRes = await bob.cnd.executeAction(decline);

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

    run();
}, 0);
