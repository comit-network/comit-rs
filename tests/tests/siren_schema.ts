/**
 * @ledger bitcoin
 * @ledger ethereum
 */

import { startAlice } from "../src/actor_test";
import "../src/schema_matcher";
import * as sirenJsonSchema from "../siren.schema.json";
import * as rootJsonSchema from "../root.schema.json";
import axios from "axios";
import * as siren from "../src/cnd_client/siren";

describe("Siren Schema", () => {
    it(
        "can-fetch-root-document-as-valid-siren",
        startAlice(async (alice) => {
            const res = await axios({
                baseURL: alice.cndHttpApiUrl(),
                url: "/",
                headers: { accept: "application/vnd.siren+json" },
            });

            expect(res.status).toBe(200);
            expect(res.data).toMatchSchema(sirenJsonSchema);
            expect(res.data.properties).toMatchSchema(rootJsonSchema);

            const body = res.data as any;
            const links = body.links;

            const swapsLink = links.find(
                (link: siren.Link) =>
                    link.rel.length === 1
                    && link.rel.includes("collection")
                    && link.class.length === 1
                    && link.class.includes("swaps"),
            );

            expect(swapsLink).toMatchObject({
                rel: ["collection"],
                class: ["swaps"],
                href: "/swaps",
            });
        }),
    );
});
