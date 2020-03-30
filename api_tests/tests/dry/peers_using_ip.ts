/**
 * @logDir peers_ip
 */

import { threeActorTest, twoActorTest } from "../../src/actor_test";
import { createDefaultSwapRequest, sleep } from "../../src/utils";
import { request } from "chai";
import { Actor } from "../../src/actors/actor";

// ******************************************** //
// Peers using ips                              //
// ******************************************** //

async function assertNoPeersAvailable(actor: Actor, message: string) {
    const peersResponse = await request(actor.cndHttpApiUrl()).get("/peers");

    expect(peersResponse.status).toEqual(200);
    try {
        expect(peersResponse.body.peers).toBeArrayOfSize(0);
    } catch (e) {
        throw new Error(message);
    }
}

async function assertPeersAvailable(alice: Actor, bob: Actor, message: string) {
    const peersResponse = await request(alice.cndHttpApiUrl()).get("/peers");

    expect(peersResponse.status).toEqual(200);

    const bobPeerId = await bob.cnd.getPeerId();
    try {
        expect(
            peersResponse.body.peers.map((peer: any) => peer.id)
        ).toIncludeAnyMembers([bobPeerId]);
    } catch (e) {
        throw new Error(message);
    }
}

describe("Peers using IP tests", () => {
    it(
        "alice-empty-peer-list",
        twoActorTest(async ({ alice }) => {
            const res = await request(alice.cndHttpApiUrl()).get("/peers");

            expect(res.status).toEqual(200);
            expect(res.body.peers).toBeArrayOfSize(0);
        })
    );

    it(
        "alice-send-request-wrong-peer-id",
        threeActorTest(async ({ alice, bob, charlie }) => {
            await assertNoPeersAvailable(
                alice,
                "[Alice] Should not yet see Bob's nor Charlie's peer id in her list of peers"
            );

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

            await assertNoPeersAvailable(
                alice,
                "[Alice] Should not see any peers because the address did not resolve to the given PeerID"
            );

            await assertNoPeersAvailable(
                bob,
                "[Bob] Should not see Alice's PeerID because she dialed to a different PeerID"
            );

            await assertNoPeersAvailable(
                charlie,
                "[Charlie] Should not see Alice's PeerID because there was no communication so far"
            );
        })
    );

    it(
        "alice-send-swap-request-to-charlie",
        threeActorTest(async ({ alice, bob, charlie }) => {
            await assertNoPeersAvailable(
                alice,
                "[Alice] Should not yet see Bob's nor Charlie's peer id in her list of peers"
            );

            // Alice send swap request to Bob
            await alice.cnd.postSwap(await createDefaultSwapRequest(charlie));

            await sleep(1000);

            await assertNoPeersAvailable(
                bob,
                "[Bob] Should not see any peer ids in his list of peers"
            );

            await assertPeersAvailable(
                alice,
                charlie,
                "[Alice] Should see Charlie's peer id in her list of peers after sending a swap request to him using his ip address"
            );

            await assertPeersAvailable(
                charlie,
                alice,
                "[Charlie] Should see Alice's peer ID in his list of peers after receiving a swap request from Alice"
            );
        })
    );
});
