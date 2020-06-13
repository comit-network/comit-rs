import { oneActorTest } from "../src/actor_test";

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
        "returns-invalid-body-for-bad-json",
        oneActorTest(async ({ alice }) => {
            const promise = alice.cnd.postSwap({
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
});
