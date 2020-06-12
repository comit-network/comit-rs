import { oneActorTest } from "../src/actor_test";
import "../src/schema_matcher";
import * as sirenJsonSchema from "../siren.schema.json";
import { siren } from "comit-sdk";
import axios from "axios";

// ******************************************** //
// Siren Schema tests                                 //
// ******************************************** //

describe("Siren Schema", () => {
    it(
        "can-fetch-root-document-as-siren",
        oneActorTest(async ({ alice }) => {
            const res = await alice.cnd.fetch("/");

            expect(res.status).toBe(200);
            expect(res.data).toMatchSchema(sirenJsonSchema);
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

            expect(typeof body.properties.id).toBe("string");
            expect(
                Array.isArray(body.properties.listen_addresses)
            ).toBeTruthy();
            // At least 2 ipv4 addresses, lookup and external interface
            expect(
                body.properties.listen_addresses.length
            ).toBeGreaterThanOrEqual(2);
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
                (link: siren.Link) =>
                    link.rel.length === 1 &&
                    link.rel.includes("collection") &&
                    link.class.length === 1 &&
                    link.class.includes("swaps")
            );

            expect(swapsLink).toMatchObject({
                rel: ["collection"],
                class: ["swaps"],
                href: "/swaps",
            });
        })
    );
});
