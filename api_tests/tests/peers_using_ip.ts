import { threeActorTest, twoActorTest } from "../src/actor_test";
import { createDefaultSwapRequest, sleep } from "../src/utils";
import { Actor } from "../src/actors/actor";

// ******************************************** //
// Peers using ips                              //
// ******************************************** //

async function assertPeersAvailable(actor: Actor, peers: Actor[]) {
    const peersResponse = await actor.cnd.fetch("/peers");
    const body = peersResponse.data as {
        peers: { id: string; endpoints: string[] }[];
    };

    const promises = peers.map(async (actor) => {
        return { id: await actor.cnd.getPeerId() };
    });

    const expectedPeers = await Promise.all(promises);

    expect(peersResponse.status).toBe(200);
    expect(body.peers).toHaveLength(peers.length);

    // We only want to check the ids
    const actualPeers = body.peers.map((actor) => {
        return { id: actor.id };
    });
    expect(actualPeers).toEqual(expect.arrayContaining(expectedPeers));
}

describe("Peers using IP tests", () => {
    it(
        "alice-empty-peer-list",
        twoActorTest(async ({ alice }) => {
            await assertPeersAvailable(alice, []);
        })
    );

    it(
        "alice-send-request-wrong-peer-id",
        threeActorTest(async ({ alice, bob, charlie }) => {
            await assertPeersAvailable(alice, []);

            // Alice send swap request to Bob
            const swapRequest = await createDefaultSwapRequest(bob);
            await alice.cnd.postSwap({
                ...swapRequest,
                peer: {
                    peer_id: "QmXfGiwNESAFWUvDVJ4NLaKYYVopYdV5HbpDSgz5TSypkb", // Random peer id on purpose to see if Bob still appears in GET /swaps using the multiaddress
                    address_hint: await bob.cnd
                        .getPeerListenAddresses()
                        .then((addresses) => addresses[0]),
                },
            });

            await sleep(1000);

            await assertPeersAvailable(alice, []);

            await assertPeersAvailable(bob, []);

            await assertPeersAvailable(charlie, []);
        })
    );

    it(
        "alice-send-swap-request-to-charlie",
        threeActorTest(async ({ alice, bob, charlie }) => {
            await assertPeersAvailable(alice, []);

            // Alice send swap request to Bob
            await alice.cnd.postSwap(await createDefaultSwapRequest(charlie));

            await sleep(1000);

            await assertPeersAvailable(bob, []);

            await assertPeersAvailable(alice, [charlie]);

            await assertPeersAvailable(charlie, [alice]);
        })
    );
});
