// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
// These test do not use the sdk so that we can test edge cases
import { threeActorTest } from "../lib/actor_test";
import { expect } from "chai";
import "chai/register-should";
import "../lib/setup_chai";
import { SwapDetails } from "comit-sdk/dist/src/cnd/cnd";
import { createDefaultSwapRequest } from "../lib/utils";

interface MatchInterface {
    id: string;
    status: string;
    state: string;
}

function toMatch(swapDetail: SwapDetails): MatchInterface {
    return {
        id: swapDetail.properties.id,
        status: swapDetail.properties.status,
        state: swapDetail.properties.state.communication.status,
    };
}

setTimeout(async function() {
    describe("SWAP requests to multiple peers", () => {
        threeActorTest(
            "[Alice] Should be able to send a swap request to Bob and Charlie",
            async function({ alice, bob, charlie }) {
                // Alice send swap request to Bob
                const aliceToBobSwapUrl = await alice.cnd.postSwap(
                    await createDefaultSwapRequest(bob)
                );

                // Alice send swap request to Charlie
                const aliceToCharlieSwapUrl = await alice.cnd.postSwap(
                    await createDefaultSwapRequest(charlie)
                );

                // fetch swap details
                const aliceToBobSwapDetails = await alice.pollSwapDetails(
                    aliceToBobSwapUrl
                );

                const aliceToCharlieSwapDetails = await alice.pollSwapDetails(
                    aliceToCharlieSwapUrl
                );

                // Bob get swap details
                const bobSwapDetails = await bob.pollSwapDetails(
                    aliceToBobSwapUrl
                );

                // Charlie get swap details
                const charlieSwapDetails = await charlie.pollSwapDetails(
                    aliceToCharlieSwapUrl
                );

                expect(
                    bobSwapDetails.properties,
                    "[Bob] should have same id as Alice"
                ).to.have.property("id", aliceToBobSwapDetails.properties.id);
                expect(
                    charlieSwapDetails.properties,
                    "[Charlie] should have same id as Alice"
                ).to.have.property(
                    "id",
                    aliceToCharlieSwapDetails.properties.id
                );

                expect(
                    [
                        aliceToBobSwapDetails,
                        aliceToCharlieSwapDetails,
                    ].map(swapDetail => toMatch(swapDetail))
                ).to.have.deep.members([
                    toMatch(bobSwapDetails),
                    toMatch(charlieSwapDetails),
                ]);
            }
        );
    });

    run();
}, 0);
