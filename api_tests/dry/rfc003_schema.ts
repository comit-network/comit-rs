import { expect, request } from "chai";
import "chai/register-should";
import { ethers } from "ethers";
import { EmbeddedRepresentationSubEntity, Entity, Link } from "../gen/siren";
import { Actor } from "../lib/actor";
import "../lib/setup_chai";
import { HarnessGlobal } from "../lib/util";
import * as sirenJsonSchema from "../siren.schema.json";
import * as swapPropertiesJsonSchema from "../swap.schema.json";

declare var global: HarnessGlobal;

(async function() {
    const alpha = {
        ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        asset: {
            name: "bitcoin",
            quantity: "100000000",
        },
        expiry: new Date("2080-06-11T23:00:00Z").getTime() / 1000,
    };

    const beta = {
        ledger: {
            name: "ethereum",
            network: "regtest",
        },
        asset: {
            name: "ether",
            quantity: ethers.utils.parseEther("10").toString(),
        },
        expiry: new Date("2080-06-11T13:00:00Z").getTime() / 1000,
    };

    const alice = new Actor("alice", global.config, global.project_root);
    const bob = new Actor("bob", global.config, global.project_root);
    const aliceFinalAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const bobCndPeerId = await bob.peerId();

    describe("Response shape", () => {
        before(async () => {
            const res = await request(alice.cndHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: alpha.ledger.name,
                        network: alpha.ledger.network,
                    },
                    beta_ledger: {
                        name: beta.ledger.name,
                        network: beta.ledger.network,
                    },
                    alpha_asset: {
                        name: alpha.asset.name,
                        quantity: alpha.asset.quantity,
                    },
                    beta_asset: {
                        name: beta.asset.name,
                        quantity: beta.asset.quantity,
                    },
                    beta_ledger_redeem_identity: aliceFinalAddress,
                    alpha_expiry: alpha.expiry,
                    beta_expiry: beta.expiry,
                    peer: bobCndPeerId,
                });

            expect(res.error).to.be.false;
            expect(res.status).to.equal(201);
        });

        it("[Alice] Response for GET /swaps is a valid siren document", async () => {
            const res = await request(alice.cndHttpApiUrl()).get("/swaps");

            expect(res.body).to.be.jsonSchema(sirenJsonSchema);
        });

        it("[Bob] Response for GET /swaps is a valid siren document", async () => {
            const res = await request(bob.cndHttpApiUrl()).get("/swaps");

            expect(res.body).to.be.jsonSchema(sirenJsonSchema);
        });

        it("[Alice] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema", async () => {
            const swapsEntity = await alice
                .pollCndUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            const selfLink = swapsEntity.links.find((link: Link) =>
                link.rel.includes("self")
            ).href;

            const swapResponse = await request(alice.cndHttpApiUrl()).get(
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
                .pollCndUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            const selfLink = swapsEntity.links.find((link: Link) =>
                link.rel.includes("self")
            ).href;

            const swapResponse = await request(bob.cndHttpApiUrl()).get(
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
