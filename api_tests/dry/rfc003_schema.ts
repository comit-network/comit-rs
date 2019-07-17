import { expect, request } from "chai";
import "chai/register-should";
import { toWei } from "web3-utils";
import { EmbeddedRepresentationSubEntity, Entity, Link } from "../gen/siren";
import { Actor } from "../lib/actor";
import "../lib/setupChai";
import { HarnessGlobal } from "../lib/util";
import * as sirenJsonSchema from "../siren.schema.json";
import * as swapPropertiesJsonSchema from "../swap.schema.json";

declare var global: HarnessGlobal;

(async function() {
    const alpha_ledger_name = "bitcoin";
    const alpha_ledger_network = "regtest";

    const beta_ledger_name = "ethereum";
    const beta_ledger_network = "regtest";

    const alpha_asset_name = "bitcoin";
    const alpha_asset_quantity = "100000000";

    const beta_asset_name = "ether";
    const beta_asset_quantity = toWei("10", "ether");

    const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    const alice = new Actor("alice", global.config, global.project_root);
    const bob = new Actor("bob", global.config, global.project_root);
    const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const bob_comit_node_address = await bob.peerId();

    describe("Response shape", () => {
        before(async () => {
            const res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: alpha_ledger_name,
                        network: alpha_ledger_network,
                    },
                    beta_ledger: {
                        name: beta_ledger_name,
                        network: beta_ledger_network,
                    },
                    alpha_asset: {
                        name: alpha_asset_name,
                        quantity: alpha_asset_quantity,
                    },
                    beta_asset: {
                        name: beta_asset_name,
                        quantity: beta_asset_quantity,
                    },
                    beta_ledger_redeem_identity: alice_final_address,
                    alpha_expiry,
                    beta_expiry,
                    peer: bob_comit_node_address,
                });

            res.error.should.equal(false);
            res.should.have.status(201);
        });

        it("[Alice] Response for GET /swaps is a valid siren document", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/swaps"
            );

            expect(res.body).to.be.jsonSchema(sirenJsonSchema);
        });

        it("[Bob] Response for GET /swaps is a valid siren document", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get("/swaps");

            expect(res.body).to.be.jsonSchema(sirenJsonSchema);
        });

        it("[Alice] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema", async () => {
            const swapsEntity = await alice
                .pollComitNodeUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            const selfLink = swapsEntity.links.find((link: Link) =>
                link.rel.includes("self")
            ).href;

            const swapResponse = await request(alice.comitNodeHttpApiUrl()).get(
                selfLink
            );
            const swapEntity = swapResponse.body as Entity;

            expect(swapEntity).to.be.jsonSchema(sirenJsonSchema);
            expect(swapEntity.properties).to.be.jsonSchema(
                swapPropertiesJsonSchema
            );
        });

        it("[Bob] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema", async () => {
            const swapsEntity = await bob
                .pollComitNodeUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            const selfLink = swapsEntity.links.find((link: Link) =>
                link.rel.includes("self")
            ).href;

            const swapResponse = await request(bob.comitNodeHttpApiUrl()).get(
                selfLink
            );
            const swapEntity = swapResponse.body as Entity;

            expect(swapEntity).to.be.jsonSchema(sirenJsonSchema);
            expect(swapEntity.properties).to.be.jsonSchema(
                swapPropertiesJsonSchema
            );
        });
    });

    run();
})();
