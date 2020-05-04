import { oneActorTest } from "../src/actor_test";
import { expect } from "chai";
import * as sirenJsonSchema from "../siren.schema.json";
import { Link } from "comit-sdk";
import axios from "axios";

// ******************************************** //
// Siren Schema tests                                 //
// ******************************************** //

describe("Siren Schema", () => {
    it(
        "can-fetch-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await alice.cnd.fetch("/");

            expect(res.status).to.equal(200);
            expect(res.data).to.be.jsonSchema(sirenJsonSchema);
        })
    );

    it(
        "returns-listen-addresses-on-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await axios({
                baseURL: alice.cndHttpApiUrl(),
                url: "/",
                headers: { accept: "application/vnd.siren+json" },
            });
            const body = res.data as any;

            expect(body.properties.id).to.be.a("string");
            expect(body.properties.listen_addresses).to.be.an("array");
            // At least 2 ipv4 addresses, lookup and external interface
            expect(body.properties.listen_addresses.length).to.be.greaterThan(
                1
            );
        })
    );

    it(
        "returns-links-to-create-swap-endpoints-on-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await axios({
                baseURL: alice.cndHttpApiUrl(),
                url: "/",
                headers: { accept: "application/vnd.siren+json" },
            });
            const body = res.data as any;
            const links = body.links;

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
