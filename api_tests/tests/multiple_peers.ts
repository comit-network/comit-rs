import { threeActorTest } from "../src/actor_test";
import { createDefaultSwapRequest } from "../src/utils";
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
    it(
        "alice-sends-swap-request-to-bob-and-charlie",
        threeActorTest(async ({ alice, bob, charlie }) => {
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
            const bobSwapDetails = await bob.pollSwapDetails(aliceToBobSwapUrl);

            // Charlie get swap details
            const charlieSwapDetails = await charlie.pollSwapDetails(
                aliceToCharlieSwapUrl
            );

            expect(bobSwapDetails.properties).toHaveProperty(
                "id",
                aliceToBobSwapDetails.properties.id
            );
            expect(charlieSwapDetails.properties).toHaveProperty(
                "id",
                aliceToCharlieSwapDetails.properties.id
            );

            expect(toMatch(aliceToBobSwapDetails)).toMatchObject(
                toMatch(bobSwapDetails)
            );
            expect(toMatch(aliceToCharlieSwapDetails)).toMatchObject(
                toMatch(charlieSwapDetails)
            );
        })
    );
});
