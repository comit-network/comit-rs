// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
import "chai/register-should";
import "../lib/setup_chai";
import { oneActorTest } from "../lib_sdk/actor_test";
import { expect, request } from "chai";
import { Actor } from "../lib_sdk/actors/actor";
import { Entity, Link } from "../gen/siren";
import * as sirenJsonSchema from "../siren.schema.json";

setTimeout(async function() {
    describe("Sanity tests", () => {
        oneActorTest(
            "[Alice] Returns 404 when you try and GET a non-existent swap",
            async function(alice: Actor) {
                const res = await request(alice.cndHttpApiUrl()).get(
                    "/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead"
                );

                expect(res).to.have.status(404);
                expect(res).to.have.header(
                    "content-type",
                    "application/problem+json"
                );
            }
        );
        oneActorTest(
            "Returns an empty list when calling GET /swaps when there are no swaps",
            async function(alice: Actor) {
                const res = await request(alice.cndHttpApiUrl()).get("/swaps");

                const body = res.body as Entity;

                expect(body.entities).to.have.lengthOf(0);
            }
        );

        oneActorTest(
            "[Alice] Returns 400 invalid body for an unsupported combination of parameters",
            async function(alice: Actor) {
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
            }
        );

        oneActorTest(
            "[Alice] Returns 400 invalid body for malformed requests",
            async function(alice: Actor) {
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
            }
        );

        oneActorTest(
            "[Alice] Should have no peers before making a swap request",
            async function(alice: Actor) {
                const res = await request(alice.cndHttpApiUrl()).get("/peers");

                expect(res).to.have.status(200);
                expect(res.body.peers).to.have.length(0);
            }
        );

        oneActorTest(
            "[Alice] Returns its peer ID and the addresses it listens on when you GET /",
            async function(alice: Actor) {
                const res = await request(alice.cndHttpApiUrl()).get("/");

                expect(res.body.id).to.be.a("string");
                expect(res.body.listen_addresses).to.be.an("array");
                // At least 2 ipv4 addresses, lookup and external interface
                expect(res.body.listen_addresses.length).to.be.greaterThan(1);
            }
        );

        oneActorTest(
            "[Alice] Response for GET / with accept header set as application/vnd.siren+json is a valid siren document",
            async function(alice: Actor) {
                const res = await request(alice.cndHttpApiUrl()).get("/");

                expect(res).to.have.status(200);
                expect(res.body).to.be.jsonSchema(sirenJsonSchema);
            }
        );

        oneActorTest(
            "[Alice] Returns its peer ID and the addresses it listens on when you GET / with accept header set as application/vnd.siren+json",
            async function(alice: Actor) {
                const res = await request(alice.cndHttpApiUrl())
                    .get("/")
                    .set("accept", "application/vnd.siren+json");

                expect(res.body.properties.id).to.be.a("string");
                expect(res.body.properties.listen_addresses).to.be.an("array");
                // At least 2 ipv4 addresses, lookup and external interface
                expect(
                    res.body.properties.listen_addresses.length
                ).to.be.greaterThan(1);
            }
        );

        oneActorTest(
            "[Alice] Returns the links for /swaps and /swaps/rfc003 when you GET / with accept header set as application/vnd.siren+json",
            async function(alice: Actor) {
                const res = await request(alice.cndHttpApiUrl())
                    .get("/")
                    .set("accept", "application/vnd.siren+json");
                const links = res.body.links;

                const swapsLink = links.find(
                    (link: Link) =>
                        link.rel.length === 1 &&
                        link.rel.includes("collection") &&
                        link.class.length === 1 &&
                        link.class.includes("swaps")
                );

                expect(swapsLink).to.be.deep.equal({
                    rel: ["collection"],
                    class: ["swaps"],
                    href: "/swaps",
                });

                const rfc003SwapsLink = links.find(
                    (link: Link) =>
                        link.rel.length === 2 &&
                        link.rel.includes("collection") &&
                        link.rel.includes("edit") &&
                        link.class.length === 2 &&
                        link.class.includes("swaps") &&
                        link.class.includes("rfc003")
                );

                expect(rfc003SwapsLink).to.be.deep.equal({
                    rel: ["collection", "edit"],
                    class: ["swaps", "rfc003"],
                    href: "/swaps/rfc003",
                });
            }
        );
    });

    run();
}, 0);
