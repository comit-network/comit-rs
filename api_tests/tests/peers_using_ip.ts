import { threeActorTest, twoActorTest } from "../src/actor_test";
import { createDefaultSwapRequest, sleep } from "../src/utils";
import { Actor } from "../src/actors/actor";
import { Rfc003Actor } from "../src/actors/rfc003_actor";

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
        threeActorTest(async (actors) => {
            const [alice, bob, carol] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
                actors.carol,
            ]);
            await assertPeersAvailable(alice.actor, []);

            // Alice send swap request to Bob
            const swapRequest = await createDefaultSwapRequest(bob);
            await alice.actor.cnd.postSwap({
                ...swapRequest,
                peer: {
                    peer_id: "QmXfGiwNESAFWUvDVJ4NLaKYYVopYdV5HbpDSgz5TSypkb", // Random peer id on purpose to see if Bob still appears in GET /swaps using the multiaddress
                    address_hint: await bob.actor.cnd
                        .getPeerListenAddresses()
                        .then((addresses) => addresses[0]),
                },
            });

            await sleep(1000);

            await assertPeersAvailable(alice.actor, []);

            await assertPeersAvailable(bob.actor, []);

            await assertPeersAvailable(carol.actor, []);
        })
    );

    it(
        "alice-send-swap-request-to-carol",
        threeActorTest(async (actors) => {
            const [alice, bob, carol] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
                actors.carol,
            ]);
            await assertPeersAvailable(alice.actor, []);

            // Alice send swap request to Bob
            await alice.actor.cnd.postSwap(
                await createDefaultSwapRequest(carol)
            );

            await sleep(1000);

            await assertPeersAvailable(bob.actor, []);

            await assertPeersAvailable(alice.actor, [carol.actor]);

            await assertPeersAvailable(carol.actor, [alice.actor]);
        })
    );
});
