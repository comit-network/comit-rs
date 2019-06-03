import { Actor } from "../lib/actor";
import { HarnessGlobal, sleep } from "../lib/util";
import { request } from "chai";
import "chai/register-should";
import { toWei } from "web3-utils";
import "../lib/setupChai";

declare var global: HarnessGlobal;

(async function() {
    const alice = new Actor("alice", global.config, global.project_root);
    const bob = new Actor("bob", global.config, global.project_root);
    const bob_peer_id = await bob.peerId();
    it("[Alice] Should not yet see  Bob peer_id in her list of peers", async () => {
        await sleep(1000);
        let res = await request(alice.comitNodeHttpApiUrl()).get("/peers");

        res.should.have.status(200);
        res.body.peers.should.not.containSubset([
            {
                id: bob_peer_id,
            },
        ]);
    });

    describe("SWAP request with address", () => {
        it("[Alice] Should be able to make a swap request via HTTP api using an ip address", async () => {
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
                        peer_id: await alice.peerId(), // Incorrect peer id on purpose to see if Bob still appears in GET /swaps
                        address: bob.comitNodelibp2pAddress(),
                    },
                });

            res.error.should.equal(false);
            res.should.have.status(201);
            const swap_location = res.header.location;
            swap_location.should.be.a("string");
        });

        it("[Alice] Should see Bob peer_id in her list of peers after sending a swap request to him using his ip address", async () => {
            await sleep(1000);
            let res = await request(alice.comitNodeHttpApiUrl()).get("/peers");

            res.should.have.status(200);
            res.body.peers.should.containSubset([
                {
                    id: bob_peer_id,
                },
            ]);
        });

        it("[Bob] Should see a new peer in his list of peers after receiving a swap request from Alice", async () => {
            let res = await request(bob.comitNodeHttpApiUrl()).get("/peers");

            res.should.have.status(200);
            res.body.peers.should.have.length(1);
        });
    });

    run();
})();
