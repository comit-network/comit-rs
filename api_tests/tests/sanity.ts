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
