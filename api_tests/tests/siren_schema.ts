import { oneActorTest } from "../src/actor_test";
import { expect, request } from "chai";
import * as sirenJsonSchema from "../siren.schema.json";
import { Link } from "comit-sdk";

// ******************************************** //
// Siren Schema tests                                 //
// ******************************************** //

describe("Siren Schema", () => {
    it(
        "can-fetch-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await request(alice.cndHttpApiUrl()).get("/");

            expect(res).to.have.status(200);
            expect(res.body).to.be.jsonSchema(sirenJsonSchema);
        })
    );

    it(
        "returns-listen-addresses-on-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await request(alice.cndHttpApiUrl())
                .get("/")
                .set("accept", "application/vnd.siren+json");

            expect(res.body.properties.id).to.be.a("string");
            expect(res.body.properties.listen_addresses).to.be.an("array");
            // At least 2 ipv4 addresses, lookup and external interface
            expect(
                res.body.properties.listen_addresses.length
            ).to.be.greaterThan(1);
        })
    );

    it(
        "returns-links-to-create-swap-endpoints-on-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
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
        })
    );
});
