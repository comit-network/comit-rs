// These are stateless tests -- they don't require any state of the comit node and they don't change it
// They are mostly about checking invalid request responses
import { expect, request } from "chai";
import "chai/register-should";
import { Entity } from "../gen/siren";
import { Actor } from "../lib/actor";
import "../lib/setupChai";
import { HarnessGlobal } from "../lib/util";

declare var global: HarnessGlobal;

const alice = new Actor("alice", global.config, global.project_root);

// the `setTimeout` forces it to be added on the event loop
// This is needed because there is no async call in the test
// And hence it does not get run without this `setTimeout`
setTimeout(async function() {
    describe("Sanity tests", () => {
        it("[Alice] Returns 404 when you try and GET a non-existent swap", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead"
            );

            res.should.have.status(404);
        });

        it("Returns a 404 for an action on a non-existent swap", async () => {
            const res = await request(alice.comitNodeHttpApiUrl())
                .post(
                    "/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead/accept"
                )
                .send({});

            res.should.have.status(404);
        });

        it("Returns an empty list when calling GET /swaps when there are no swaps", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/swaps"
            );

            const body = res.body as Entity;

            expect(body.entities).to.have.lengthOf(0);
        });

        it("[Alice] Returns 400 Swap not supported for an unsupported combination of parameters", async () => {
            const res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: "Thomas' wallet",
                    },
                    beta_ledger: {
                        name: "Higher-Dimension", // This is the coffee place downstairs
                    },
                    alpha_asset: {
                        name: "AUD",
                        quantity: "3.5",
                    },
                    beta_asset: {
                        name: "Espresso",
                        "double-shot": true,
                    },
                    alpha_ledger_refund_identity: "",
                    beta_ledger_redeem_identity: "",
                    alpha_expiry: 123456789,
                    beta_expiry: 123456789,
                    peer: "QmPRNaiDUcJmnuJWUyoADoqvFotwaMRFKV2RyZ7ZVr1fqd",
                });

            res.should.have.status(400);
            res.body.title.should.equal("Swap not supported.");
        });

        it("[Alice] Returns 400 bad request for malformed requests", async () => {
            const res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    garbage: true,
                });

            res.should.have.status(400);
            res.body.title.should.equal("Bad Request");
        });

        it("[Alice] Should have no peers before making a swap request", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/peers"
            );

            res.should.have.status(200);
            res.body.peers.should.have.length(0);
        });

        it("[Alice] Returns its peer ID when you GET /", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get("/");
            expect(res.status).to.equal(200);
            expect(res.body.id).to.be.a("string");
        });

        it("[Alice] Returns the addresses it listens on when you GET /", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get("/");
            expect(res.status).to.equal(200);
            expect(res.body.listen_addresses).to.be.an("array");
            // At least 2 ipv4 addresses, lookup and external interface
            expect(res.body.listen_addresses.length).to.be.greaterThan(2);
        });
    });

    run();
}, 0);
