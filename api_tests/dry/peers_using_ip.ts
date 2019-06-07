import { Actor } from "../lib/actor";
import { HarnessGlobal, sleep } from "../lib/util";
import { expect, request } from "chai";
import { toWei } from "web3-utils";
import "../lib/setupChai";

declare var global: HarnessGlobal;

(async () => {
    const alice = new Actor("alice", global.config, global.project_root);
    const bob = new Actor("bob", global.config, global.project_root);
    const charlie = new Actor("charlie", global.config, global.project_root);
    const alicePeerId = await alice.peerId();
    const bobMultiAddress = bob.comitNodeNetworkListenAddress();
    const charliePeerId = await charlie.peerId();
    const charlieMultiAddress = charlie.comitNodeNetworkListenAddress();

    describe("SWAP request with address", () => {
        it("[Alice] Should not yet see Bob's peer id in her list of peers", async () => {
            let res = await request(alice.comitNodeHttpApiUrl()).get("/peers");

            expect(res.status).to.equal(200);
            expect(res.body.peers).to.be.empty;
        });

        it("[Alice] Should be able to make a swap request via HTTP api using a random peer id and Bob's ip address", async () => {
            let res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: "bitcoin",
                        network: "regtest",
                    },
                    beta_ledger: {
                        name: "ethereum",
                        network: "regtest",
                    },
                    alpha_asset: {
                        name: "bitcoin",
                        quantity: "100000000",
                    },
                    beta_asset: {
                        name: "ether",
                        quantity: toWei("10", "ether"),
                    },
                    beta_ledger_redeem_identity:
                        "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                    alpha_expiry:
                        new Date("2080-06-11T23:00:00Z").getTime() / 1000,
                    beta_expiry:
                        new Date("2080-06-11T13:00:00Z").getTime() / 1000,
                    peer: {
                        peer_id:
                            "QmXfGiwNESAFWUvDVJ4NLaKYYVopYdV5HbpDSgz5TSypkb", // Random peer id on purpose to see if Bob still appears in GET /swaps using the multiaddress
                        address_hint: bobMultiAddress,
                    },
                });

            expect(res.error).to.be.false;
            expect(res.status).to.equal(201);
            expect(res.header.location).to.be.a("string");
        });

        it("[Alice] Should not see any peers because the address did not resolve to the given PeerID", async () => {
            await sleep(1000);
            let res = await request(alice.comitNodeHttpApiUrl()).get("/peers");

            expect(res.status).to.equal(200);
            expect(res.body.peers).to.be.empty;
        });

        it("[Bob] Should not see Alice's PeerID because she dialed to a different PeerID", async () => {
            let res = await request(bob.comitNodeHttpApiUrl()).get("/peers");

            expect(res.status).to.equal(200);
            expect(res.body.peers).to.be.empty;
        });

        it("[Alice] Should not yet see Charlie's peer id in her list of peers", async () => {
            let res = await request(alice.comitNodeHttpApiUrl()).get("/peers");

            expect(res.status).to.equal(200);
            expect(res.body.peers).to.not.containSubset([
                {
                    id: charliePeerId,
                },
            ]);
        });

        it("[Alice] Should be able to make a swap request via HTTP api to Charlie using his peer ID and his ip address", async () => {
            let res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: "bitcoin",
                        network: "regtest",
                    },
                    beta_ledger: {
                        name: "ethereum",
                        network: "regtest",
                    },
                    alpha_asset: {
                        name: "bitcoin",
                        quantity: "100000000",
                    },
                    beta_asset: {
                        name: "ether",
                        quantity: toWei("10", "ether"),
                    },
                    beta_ledger_redeem_identity:
                        "0x00a329c0648769a73afac7f9381e08fb43dbea72",
                    alpha_expiry:
                        new Date("2080-06-11T23:00:00Z").getTime() / 1000,
                    beta_expiry:
                        new Date("2080-06-11T13:00:00Z").getTime() / 1000,
                    peer: {
                        peer_id: charliePeerId,
                        address_hint: charlieMultiAddress,
                    },
                });

            expect(res.error).to.be.false;
            expect(res.status).to.equal(201);
            expect(res.header.location).to.be.a("string");
        });

        it("[Alice] Should see Charlie's peer id in her list of peers after sending a swap request to him using his ip address", async () => {
            await sleep(1000);
            let res = await request(alice.comitNodeHttpApiUrl()).get("/peers");

            expect(res.status).to.equal(200);
            expect(res.body.peers).to.containSubset([
                {
                    id: charliePeerId,
                },
            ]);
        });

        it("[Charlie] Should see Alice's peer ID in his list of peers after receiving a swap request from Alice", async () => {
            let res = await request(charlie.comitNodeHttpApiUrl()).get(
                "/peers"
            );

            expect(res.status).to.equal(200);
            expect(res.body.peers).to.containSubset([
                {
                    id: alicePeerId,
                },
            ]);
        });
    });

    run();
})();
