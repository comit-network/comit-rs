import { oneActorTest, twoActorTest } from "../src/actor_test";
import SwapFactory from "../src/actors/swap_factory";

// ******************************************** //
// Sanity tests                                 //
// ******************************************** //

describe("Sanity", () => {
    it(
        "invalid-swap-yields-404",
        oneActorTest(async ({ alice }) => {
            const promise = alice.cnd.fetch(
                "/swaps/deadbeef-dead-beef-dead-deadbeefdead"
            );

            await expect(promise).rejects.toMatchObject({
                status: 404,
                title: "Swap not found.",
            });
        })
    );

    it(
        "returns-invalid-body-for-bad-json-herc20-halbit",
        oneActorTest(async ({ alice }) => {
            const promise = alice.createHerc20Halbit({
                // @ts-ignore
                garbage: true,
            });

            await expect(promise).rejects.toMatchObject({
                status: 400,
                title: "Invalid body.",
            });
        })
    );

    it(
        "returns-invalid-body-for-bad-json-halbit-herc20",
        oneActorTest(async ({ alice }) => {
            const promise = alice.createHalbitHerc20({
                // @ts-ignore
                garbage: true,
            });

            await expect(promise).rejects.toMatchObject({
                status: 400,
                title: "Invalid body.",
            });
        })
    );

    it(
        "returns-invalid-body-for-bad-json-herc20-hbit",
        oneActorTest(async ({ alice }) => {
            const promise = alice.createHerc20Hbit({
                // @ts-ignore
                garbage: true,
            });

            await expect(promise).rejects.toMatchObject({
                status: 400,
                title: "Invalid body.",
            });
        })
    );

    it(
        "returns-invalid-body-for-bad-json-hbit-herc20",
        oneActorTest(async ({ alice }) => {
            const promise = alice.createHbitHerc20({
                // @ts-ignore
                garbage: true,
            });

            await expect(promise).rejects.toMatchObject({
                status: 400,
                title: "Invalid body.",
            });
        })
    );

    it(
        "alice-has-empty-peer-list",
        oneActorTest(async ({ alice }) => {
            const promise = alice.cnd.fetch("/peers");

            await expect(promise).resolves.toMatchObject({
                status: 200,
                data: { peers: [] },
            });
        })
    );

    it(
        "returns-listen-addresses-on-root-document",
        oneActorTest(async ({ alice }) => {
            const res = await alice.cnd.fetch("/");

            const body = res.data as { id: string; listen_addresses: string[] };

            expect(typeof body.id).toEqual("string");
            expect(body.id).toBeTruthy();
            // At least 2 ipv4 addresses, lookup and external interface
            expect(body.listen_addresses.length).toBeGreaterThanOrEqual(2);
        })
    );

    it(
        "create-herc20-halbit-returns-bad-request-when-no-lnd-node",
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
        "create-halbit-herc20-returns-bad-request-when-no-lnd-node",
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
