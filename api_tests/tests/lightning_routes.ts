import { twoActorTest } from "../src/actor_test";
import SwapFactory from "../src/actors/swap_factory";

// ******************************************** //
// Lightning routes                               //
// ******************************************** //

describe("Lightning routes tests", () => {
    it(
        "create-herc20-halbit-returns-bad-request",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).herc20Halbit;

            const expectedProblem = {
                status: 400,
                title: "lightning is not configured.",
                detail:
                    "lightning ledger is not properly configured, swap involving this ledger are not available.",
            };

            await expect(
                alice.createHerc20Halbit(bodies.alice)
            ).rejects.toMatchObject(expectedProblem);
            await expect(
                bob.createHerc20Halbit(bodies.bob)
            ).rejects.toMatchObject(expectedProblem);
        })
    );

    it(
        "create-halbit-herc20-returns-bad-request",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).halbitHerc20;

            const expectedProblem = {
                status: 400,
                title: "lightning is not configured.",
                detail:
                    "lightning ledger is not properly configured, swap involving this ledger are not available.",
            };

            await expect(
                alice.createHalbitHerc20(bodies.alice)
            ).rejects.toMatchObject(expectedProblem);
            await expect(
                bob.createHalbitHerc20(bodies.bob)
            ).rejects.toMatchObject(expectedProblem);
        })
    );
});
