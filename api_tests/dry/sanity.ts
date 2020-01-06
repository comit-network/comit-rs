// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
import { expect, request } from "chai";
import "chai/register-should";
import { Entity, Link } from "../gen/siren";
import { Actor } from "../lib/actor";
import "../lib/setup_chai";
import * as sirenJsonSchema from "../siren.schema.json";

const alice = new Actor("alice");

// the `setTimeout` forces it to be added on the event loop
// This is needed because there is no async call in the test
// And hence it does not get run without this `setTimeout`
setTimeout(async function() {
    describe("Sanity tests", () => {
        it("[Alice] Returns 404 when you try and GET a non-existent swap", async () => {
            const res = await request(alice.cndHttpApiUrl()).get(
                "/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead"
            );

            expect(res).to.have.status(404);
            expect(res).to.have.header(
                "content-type",
                "application/problem+json"
            );
        });

        it("Returns an empty list when calling GET /swaps when there are no swaps", async () => {
            const res = await request(alice.cndHttpApiUrl()).get("/swaps");

            const body = res.body as Entity;

            expect(body.entities).to.have.lengthOf(0);
        });

        it("[Alice] Returns 400 invalid body for an unsupported combination of parameters", async () => {
            const res = await request(alice.cndHttpApiUrl())
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

            expect(res).to.have.status(400);
            expect(res).to.have.header(
                "content-type",
                "application/problem+json"
            );
            expect(res.body.title).to.equal("Invalid body.");
        });

        it("[Alice] Returns 400 invalid body for malformed requests", async () => {
            const res = await request(alice.cndHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    garbage: true,
                });

            expect(res).to.have.status(400);
            expect(res).to.have.header(
                "content-type",
                "application/problem+json"
            );
            expect(res.body.title).to.equal("Invalid body.");
        });

        it("[Alice] Should have no peers before making a swap request", async () => {
            const res = await request(alice.cndHttpApiUrl()).get("/peers");

            expect(res).to.have.status(200);
            expect(res.body.peers).to.have.length(0);
        });

        it("[Alice] Response for GET / is a valid siren document", async () => {
            const res = await request(alice.cndHttpApiUrl()).get("/");

            expect(res).to.have.status(200);
            expect(res.body).to.be.jsonSchema(sirenJsonSchema);
        });

        it("[Alice] Returns its peer ID when you GET /", async () => {
            const res = await request(alice.cndHttpApiUrl()).get("/");

            expect(res.body.properties.id).to.be.a("string");
        });

        it("[Alice] Returns the links for /swaps and /swaps/rfc003 when you GET /", async () => {
            const res = await request(alice.cndHttpApiUrl()).get("/");
            const links = res.body.links;

            const swapsLink = links.find(
                (link: Link) => link.href === "/swaps"
            );

            expect(swapsLink).to.be.deep.equal({
                rel: ["collection"],
                class: ["swaps"],
                href: "/swaps",
            });

            const rfc003SwapsLink = links.find(
                (link: Link) => link.href === "/swaps/rfc003"
            );

            expect(rfc003SwapsLink).to.be.deep.equal({
                rel: ["collection", "edit"],
                class: ["swaps", "rfc003"],
                href: "/swaps/rfc003",
            });
        });
    });

    run();
}, 0);
