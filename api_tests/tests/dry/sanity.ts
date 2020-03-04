/**
 * @logDir sanity
 */

import { oneActorTest } from "../../lib/actor_test";
import { expect, request } from "chai";
import { Entity, Link } from "../../gen/siren";
import * as sirenJsonSchema from "../../siren.schema.json";

// ******************************************** //
// Sanity tests                                 //
// ******************************************** //

describe("Sanity - peers using IP", () => {
    it("invalid-swap-yields-404", async function() {
        await oneActorTest("invalid-swap-yields-404", async function({
            alice,
        }) {
            const res = await request(alice.cndHttpApiUrl()).get(
                "/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead"
            );

            expect(res).to.have.status(404);
            expect(res).to.have.header(
                "content-type",
                "application/problem+json"
            );
        });
    });

    it("empty-swap-list-after-startup", async function() {
        await oneActorTest("empty-swap-list-after-startup", async function({
            alice,
        }) {
            const res = await request(alice.cndHttpApiUrl()).get("/swaps");

            const body = res.body as Entity;

            expect(body.entities).to.have.lengthOf(0);
        });
    });

    it("bad-request-for-invalid-swap-combination", async function() {
        await oneActorTest(
            "bad-request-for-invalid-swap-combination",
            async function({ alice }) {
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
    });
    it("returns-invalid-body-for-bad-json", async function() {
        await oneActorTest("returns-invalid-body-for-bad-json", async function({
            alice,
        }) {
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
    });
    it("alice-has-empty-peer-list", async function() {
        await oneActorTest("alice-has-empty-peer-list", async function({
            alice,
        }) {
            const res = await request(alice.cndHttpApiUrl()).get("/peers");

            expect(res).to.have.status(200);
            expect(res.body.peers).to.have.length(0);
        });
    });
    it("returns-listen-addresses-on-root-document", async function() {
        await oneActorTest(
            "returns-listen-addresses-on-root-document",
            async function({ alice }) {
                const res = await request(alice.cndHttpApiUrl()).get("/");

                expect(res.body.id).to.be.a("string");
                expect(res.body.listen_addresses).to.be.an("array");
                // At least 2 ipv4 addresses, lookup and external interface
                expect(res.body.listen_addresses.length).to.be.greaterThan(1);
            }
        );
    });
    it("can-fetch-root-document-as-siren", async function() {
        await oneActorTest("can-fetch-root-document-as-siren", async function({
            alice,
        }) {
            const res = await request(alice.cndHttpApiUrl()).get("/");

            expect(res).to.have.status(200);
            expect(res.body).to.be.jsonSchema(sirenJsonSchema);
        });
    });
    it("returns-listen-addresses-on-root-document-as-siren", async function() {
        await oneActorTest(
            "returns-listen-addresses-on-root-document-as-siren",
            async function({ alice }) {
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
    });
    it("returns-links-to-create-swap-endpoints-on-root-document-as-siren", async function() {
        await oneActorTest(
            "returns-links-to-create-swap-endpoints-on-root-document-as-siren",
            async function({ alice }) {
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
});
