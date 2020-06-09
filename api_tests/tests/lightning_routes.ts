import { oneActorTest, twoActorTest } from "../src/actor_test";
import SwapFactory from "../src/actors/swap_factory";
import { HalbitHerc20Payload, Herc20HalbitPayload } from "../src/payload";

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

    it(
        "create-herc20-halbit-returns-invalid-body",
        twoActorTest(async ({ alice }) => {
            await expect(
                alice.createHerc20Halbit({} as Herc20HalbitPayload)
            ).rejects.toThrow("Invalid body.");
        })
    );

    it(
        "create-halbit-herc20-returns-invalid-body",
        twoActorTest(async ({ alice }) => {
            await expect(
                alice.createHalbitHerc20({} as HalbitHerc20Payload)
            ).rejects.toThrow("Invalid body.");
        })
    );

    it(
        "get-swap-with-non-existent-id-yields-swap-not-found",
        oneActorTest(async ({ alice }) => {
            try {
                await alice.cnd.fetch(
                    "/swaps/deadbeef-dead-beef-dead-deadbeefdead"
                );
            } catch (error) {
                const expectedProblem = {
                    status: 404,
                    title: "Swap not found.",
                };

                expect(error).toMatchObject(expectedProblem);
            }
        })
    );
});
