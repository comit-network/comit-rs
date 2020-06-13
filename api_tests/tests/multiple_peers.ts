import { threeActorTest } from "../src/actor_test";
import { createDefaultSwapRequest } from "../src/utils";
import { SwapDetails } from "comit-sdk";
import { Rfc003Actor } from "../src/actors/rfc003_actor";

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
        "alice-sends-swap-request-to-bob-and-carol",
        threeActorTest(async (actors) => {
            const [alice, bob, carol] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
                actors.carol,
            ]);
            // Alice send swap request to Bob
            const aliceToBobSwapUrl = await alice.actor.cnd.postSwap(
                await createDefaultSwapRequest(bob)
            );

            // Alice send swap request to Carol
            const aliceToCarolSwapUrl = await alice.actor.cnd.postSwap(
                await createDefaultSwapRequest(carol)
            );

            // fetch swap details
            const aliceToBobSwapDetails = await alice.pollSwapDetails(
                aliceToBobSwapUrl
            );

            const aliceToCarolSwapDetails = await alice.pollSwapDetails(
                aliceToCarolSwapUrl
            );

            // Bob get swap details
            const bobSwapDetails = await bob.pollSwapDetails(aliceToBobSwapUrl);

            // Carol get swap details
            const carolSwapDetails = await carol.pollSwapDetails(
                aliceToCarolSwapUrl
            );

            expect(bobSwapDetails.properties).toHaveProperty(
                "id",
                aliceToBobSwapDetails.properties.id
            );
            expect(carolSwapDetails.properties).toHaveProperty(
                "id",
                aliceToCarolSwapDetails.properties.id
            );

            expect(toMatch(aliceToBobSwapDetails)).toMatchObject(
                toMatch(bobSwapDetails)
            );
            expect(toMatch(aliceToCarolSwapDetails)).toMatchObject(
                toMatch(carolSwapDetails)
            );
        })
    );
});
