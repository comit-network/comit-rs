import { threeActorTest, twoActorTest } from "../src/actor_test";
import { createDefaultSwapRequest, sleep } from "../src/utils";
import { Actor } from "../src/actors/actor";

// ******************************************** //
// Peers using ips                              //
// ******************************************** //

async function assertNoPeersAvailable(actor: Actor) {
    const peersResponse = await actor.cnd.fetch("/peers");
    const body = peersResponse.data as {
        peers: { id: string; endpoints: string[] }[];
    };

    expect(peersResponse.status).toBe(200);
    expect(body.peers).toHaveLength(0);
}

async function assertPeersAvailable(alice: Actor, bob: Actor) {
    const peersResponse = await alice.cnd.fetch("/peers");
    const body = peersResponse.data as {
        peers: { id: string; endpoints: string[] }[];
    };

    expect(peersResponse.status).toBe(200);
    expect(body.peers[0].id).toBe(await bob.cnd.getPeerId());
}

describe("Peers using IP tests", () => {
    it(
        "alice-empty-peer-list",
        twoActorTest(async ({ alice }) => {
            await assertNoPeersAvailable(alice);
        })
    );

    it(
        "alice-send-request-wrong-peer-id",
        threeActorTest(async ({ alice, bob, charlie }) => {
            await assertNoPeersAvailable(alice);

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

            await assertNoPeersAvailable(alice);

            await assertNoPeersAvailable(bob);

            await assertNoPeersAvailable(charlie);
        })
    );

    it(
        "alice-send-swap-request-to-charlie",
        threeActorTest(async ({ alice, bob, charlie }) => {
            await assertNoPeersAvailable(alice);

            // Alice send swap request to Bob
            await alice.cnd.postSwap(await createDefaultSwapRequest(charlie));

            await sleep(1000);

            await assertNoPeersAvailable(bob);

            await assertPeersAvailable(alice, charlie);

            await assertPeersAvailable(charlie, alice);
        })
    );
});
