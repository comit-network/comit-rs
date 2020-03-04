/**
 * @logDir multiple_peers
 */

import { threeActorTest } from "../../lib/actor_test";
import { createDefaultSwapRequest } from "../../lib/utils";
import { expect } from "chai";
import { SwapDetails } from "comit-sdk";

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

// ******************************************** //
// Multiple peers                               //
// ******************************************** //
describe("Multiple peers tests", () => {
    it("alice-sends-swap-request-to-bob-and-charlie", async function() {
        await threeActorTest(
            "alice-sends-swap-request-to-bob-and-charlie",
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
});
