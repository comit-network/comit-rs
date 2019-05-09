import {Actor} from "../lib/actor";
import {HarnessGlobal} from "../lib/util";
import chai from "chai";
import {expect} from "chai";
import utils from "web3-utils";
import {EmbeddedRepresentationSubEntity, Entity} from "../gen/siren";
import sirenJsonSchema from "../siren.schema.json";
import swapPropertiesJsonSchema from "../swap.schema.json";
import chaiHttp = require("chai-http");
import chaiJsonSchema = require("chai-json-schema");

chai.use(chaiHttp);
chai.use(chaiJsonSchema);
chai.should();

declare var global: HarnessGlobal;

(async function () {
    const alpha_ledger_name = "bitcoin";
    const alpha_ledger_network = "regtest";

    const beta_ledger_name = "ethereum";
    const beta_ledger_network = "regtest";

    const alpha_asset_name = "bitcoin";
    const alpha_asset_quantity = "100000000";

    const beta_asset_name = "ether";
    const beta_asset_quantity = utils.toWei("10", "ether");

    const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    const alice = new Actor("alice", global.config, global.project_root, {
        ethConfig: global.ledgers_config.ethereum,
    });
    const bob = new Actor("bob", global.config, global.project_root, {
        ethConfig: global.ledgers_config.ethereum,
    });
    const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const bob_comit_node_address = await bob.peerId();

    describe("Response shape", () => {

        before(async () => {
            let res = await chai
                .request(alice.comit_node_url())
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
                    alpha_expiry: alpha_expiry,
                    beta_expiry: beta_expiry,
                    peer: bob_comit_node_address,
                });

            res.error.should.equal(false);
            res.should.have.status(201);
        });

        it('[Alice] Response for GET /swaps is a valid siren document', async () => {

            let res = await chai.request(alice.comit_node_url()).get("/swaps");

            expect(res.body).should.be.jsonSchema(sirenJsonSchema);
        });

        it('[Bob] Response for GET /swaps is a valid siren document', async () => {

            let res = await chai.request(bob.comit_node_url()).get("/swaps");

            expect(res.body).should.be.jsonSchema(sirenJsonSchema);
        });

        it('[Alice] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema', async () => {

            let swapsResponse = await chai.request(alice.comit_node_url()).get("/swaps");
            let swapsEntity = swapsResponse.body as Entity;

            expect(swapsEntity.entities).to.have.length.greaterThan(0);


            let selfLink = (swapsEntity.entities[0] as EmbeddedRepresentationSubEntity).links.find(link => link.class.includes("self")).href;

            let swapResponse = await chai.request(alice.comit_node_url()).get(selfLink);
            let swapEntity = swapResponse.body as Entity;

            expect(swapEntity).should.be.jsonSchema(sirenJsonSchema);
            expect(swapEntity.properties).should.be.jsonSchema(swapPropertiesJsonSchema);
        });

        it('[Bob] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema', async () => {

            let swapsResponse = await chai.request(bob.comit_node_url()).get("/swaps");
            let swapsEntity = swapsResponse.body as Entity;

            expect(swapsEntity.entities).to.have.length.greaterThan(0);


            let selfLink = (swapsEntity.entities[0] as EmbeddedRepresentationSubEntity).links.find(link => link.class.includes("self")).href;

            let swapResponse = await chai.request(bob.comit_node_url()).get(selfLink);
            let swapEntity = swapResponse.body as Entity;

            expect(swapEntity).should.be.jsonSchema(sirenJsonSchema);
            expect(swapEntity.properties).should.be.jsonSchema(swapPropertiesJsonSchema);
        });

    });

    run();
})();
