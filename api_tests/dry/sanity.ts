// These are stateless tests -- they don't require any state of the comit node and they don't change it
// They are mostly about checking invalid request responses
import { Actor } from "../lib/actor";
import * as chai from "chai";
import { HarnessGlobal } from "../lib/util";

import chaiHttp = require("chai-http");

chai.use(chaiHttp);
const should = chai.should();

declare var global: HarnessGlobal;

const alice = new Actor("alice", global.config, global.test_root);
const bob = new Actor("bob", global.config, global.test_root);

describe("Sanity tests", () => {
    it("[Alice] Returns 404 when you try and GET a non-existent swap", async () => {
        await chai
            .request(alice.comit_node_url())
            .get("/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead")
            .then(res => {
                res.should.have.status(404);
            });
    });

    it("Returns a 404 for an action on a non-existent swap", async () => {
        return chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead/accept")
            .send({})
            .then(res => {
                res.should.have.status(404);
            });
    });

    it("Returns an empty list when calling GET /swaps when there are no swaps", async () => {
        await chai
            .request(alice.comit_node_url())
            .get("/swaps")
            .then(res => {
                let swaps = res.body._embedded.swaps;
                swaps.should.be.an("array");
                swaps.should.have.lengthOf(0);
            });
    });

    it("[Alice] Returns 400 swap-not-supported for an unsupported combination of parameters", async () => {
        await chai
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
            })
            .then(res => {
                res.should.have.status(400);
                res.body.title.should.equal("swap-not-supported");
            });
    });

    it("[Alice] Sets appropriate CORS headers", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get("/swaps")
            .set("Origin", "localhost:3000");

        res.should.have.status(200);
        res.should.have.header("access-control-allow-origin", "localhost:3000");
    });

    it("[Alice] Returns 400 bad request for malformed requests", async () => {
        await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                garbage: true,
            })
            .then(res => {
                res.should.have.status(400);
                res.body.title.should.equal("Bad Request");
            });
    });

    it("[Alice] Should have no peers before making a swap request", async () => {
        await chai
            .request(alice.comit_node_url())
            .get("/peers")
            .then(res => {
                res.should.have.status(200);
                res.body.peers.should.have.length(0);
            });
    });
});
