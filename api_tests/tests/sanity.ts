import { oneActorTest } from "../src/actor_test";
import { expect, request } from "chai";
import { Entity } from "comit-sdk";

// ******************************************** //
// Sanity tests                                 //
// ******************************************** //

describe("Sanity", () => {
    it(
        "invalid-swap-yields-404",
        oneActorTest(async ({ alice }) => {
            const res = await request(alice.cndHttpApiUrl()).get(
                "/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead"
            );

            expect(res).to.have.status(404);
            expect(res).to.have.header(
                "content-type",
                "application/problem+json"
            );
        })
    );

    it(
        "empty-swap-list-after-startup",
        oneActorTest(async ({ alice }) => {
            const res = await request(alice.cndHttpApiUrl()).get("/swaps");

            const body = res.body as Entity;

            expect(body.entities).to.have.lengthOf(0);
        })
    );

    it(
        "bad-request-for-invalid-swap-combination",
        oneActorTest(async ({ alice }) => {
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
        })
    );

    it(
        "returns-invalid-body-for-bad-json",
        oneActorTest(async ({ alice }) => {
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
        })
    );

    it(
        "alice-has-empty-peer-list",
        oneActorTest(async ({ alice }) => {
            const res = await request(alice.cndHttpApiUrl()).get("/peers");

            expect(res).to.have.status(200);
            expect(res.body.peers).to.have.length(0);
        })
    );

    it(
        "returns-listen-addresses-on-root-document",
        oneActorTest(async ({ alice }) => {
            const res = await request(alice.cndHttpApiUrl()).get("/");

            expect(res.body.id).to.be.a("string");
            expect(res.body.listen_addresses).to.be.an("array");
            // At least 2 ipv4 addresses, lookup and external interface
            expect(res.body.listen_addresses.length).to.be.greaterThan(1);
        })
    );
});
