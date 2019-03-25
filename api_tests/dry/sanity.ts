// These are stateless tests -- they don't require any state of the comit node and they don't change it
// They are mostly about checking invalid request responses
import { Actor } from "../lib/actor";
import * as chai from "chai";
import { HarnessGlobal, sleep } from "../lib/util";

import chaiHttp = require("chai-http");

chai.use(chaiHttp);
const should = chai.should();

declare var global: HarnessGlobal;

const alice = new Actor("alice", global.config, global.test_root);

// the `setTimeout` forces it to be added on the event loop
// This is needed because there is no async call in the test
// And hence it does not get run without this `setTimeout`
setTimeout(async function() {
    describe("Sanity tests", () => {
        it("[Alice] Returns 404 when you try and GET a non-existent swap", async () => {
            let res = await chai
                .request(alice.comit_node_url())
                .get("/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead");

            res.should.have.status(404);
        });

        it("Returns a 404 for an action on a non-existent swap", async () => {
            let res = await chai
                .request(alice.comit_node_url())
                .post(
                    "/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead/accept"
                )
                .send({});
            res.should.have.status(404);
        });

        it("Returns an empty list when calling GET /swaps when there are no swaps", async () => {
            let res = await chai.request(alice.comit_node_url()).get("/swaps");

            let swaps = res.body._embedded.swaps;
            swaps.should.be.an("array");
            swaps.should.have.lengthOf(0);
        });

        it("[Alice] Returns 400 swap-not-supported for an unsupported combination of parameters", async () => {
            let res = await chai
                .request(alice.comit_node_url())
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
                    peer: "0.0.0.0",
                });
            res.should.have.status(400);
            res.body.title.should.equal("swap-not-supported");
        });

        it("Returns 403 'Forbidden for invalid origins or headers' for invalid preflight OPTIONS /swaps request for GET", async () => {
            let res = await chai
                .request(alice.comit_node_url())
                .options("/swaps")
                .set("Origin", "http://localhost:8080")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "GET");
            res.should.have.status(403);
        });

        it("Returns 200 OK for preflight OPTIONS /swaps request for GET", async () => {
            let res = await chai
                .request(alice.comit_node_url())
                .options("/swaps")
                .set("Origin", "http://localhost:3000")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "GET");
            res.should.have.status(200);
        });

        it("Returns 403 'Forbidden for invalid origins or headers' for invalid preflight OPTIONS /swaps/rfc003 request for POST", async () => {
            let res = await chai
                .request(alice.comit_node_url())
                .options("/swaps/rfc003")
                .set("Origin", "http://localhost:8080")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "POST");
            res.should.have.status(403);
        });

        it("Returns 200 OK for preflight OPTIONS /swaps/rfc003 request for POST", async () => {
            let res = await chai
                .request(alice.comit_node_url())
                .options("/swaps/rfc003")
                .set("Origin", "http://localhost:3000")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "POST");
            res.should.have.status(200);
        });

        it("[Alice] Sets appropriate CORS headers", async () => {
            let res = await chai
                .request(alice.comit_node_url())
                .get("/swaps")
                .set("Origin", "http://localhost:3000");

            res.should.have.status(200);
            res.should.have.header(
                "access-control-allow-origin",
                "http://localhost:3000"
            );
        });

        it("[Alice] Returns 400 bad request for malformed requests", async () => {
            let res = await chai
                .request(alice.comit_node_url())
                .post("/swaps/rfc003")
                .send({
                    garbage: true,
                });
            res.should.have.status(400);
            res.body.title.should.equal("Bad Request");
        });

        it("[Alice] Should have no peers before making a swap request", async () => {
            let res = await chai.request(alice.comit_node_url()).get("/peers");
            res.should.have.status(200);
            res.body.peers.should.have.length(0);
        });
    });

    run();
}, 0);
