/**
 * @ledger bitcoin
 * @ledger ethereum
 */

import { startAlice, startAliceAndBob } from "../src/actor_test";
import SwapFactory from "../src/swap_factory";
import { sleep } from "../src/utils";
import "../src/schema_matcher";
import * as sirenJsonSchema from "../siren.schema.json";
import * as rootJsonSchema from "../root.schema.json";
import axios from "axios";
import { SwapEntity } from "../src/cnd_client/payload";
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

    it(
        "get-single-swap-is-valid-siren",
        startAliceAndBob(async ([alice, bob]) => {
            const bodies = (await SwapFactory.newSwap(alice, bob)).hbitHerc20;

            await alice.createHbitHerc20Swap(bodies.alice);
            await bob.createHbitHerc20Swap(bodies.bob);

            // Wait for the announce protocol to complete.
            await sleep(2000);

            // For now we just assert that the document returned by "/swaps/:id" is a valid siren object.

            const responseAlice = await alice.cnd.fetch<SwapEntity>(
                alice.swap.self
            );
            expect(responseAlice.status).toEqual(200);
            expect(responseAlice.data).toMatchSchema(sirenJsonSchema);

            const responseBob = await bob.cnd.fetch<SwapEntity>(bob.swap.self);
            expect(responseBob.status).toEqual(200);
            expect(responseBob.data).toMatchSchema(sirenJsonSchema);
        })
    );
});
