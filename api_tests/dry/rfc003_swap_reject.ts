import { expect, request } from "chai";
import "chai/register-should";
import { toWei } from "web3-utils";
import { EmbeddedRepresentationSubEntity, Entity } from "../gen/siren";
import { Actor } from "../lib/actor";
import "../lib/setupChai";
import { HarnessGlobal, sleep } from "../lib/util";
import * as swapPropertiesJsonSchema from "../swap.schema.json";

declare var global: HarnessGlobal;

(async function() {
    const alpha_ledger_name = "bitcoin";
    const alpha_ledger_network = "regtest";

    const beta_ledger_name = "ethereum";
    const beta_ledger_network = "regtest";

    const alpha_asset_name = "bitcoin";
    const alpha_asset_reasonable_quantity = "100000000";
    const alpha_asset_stingy_quantity = "100";

    const beta_asset_name = "ether";
    const beta_asset_quantity = toWei("10", "ether");

    const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    const alice = new Actor("alice", global.config, global.project_root);
    const bob = new Actor("bob", global.config, global.project_root);
    const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const bob_comit_node_address = await bob.peerId();

    describe("SWAP request REJECTED", () => {
        let alice_reasonable_swap_href: string;
        it("[Alice] Should be able to make first swap request via HTTP api", async () => {
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
                        quantity: alpha_asset_reasonable_quantity,
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
            const swap_location = res.header.location;
            swap_location.should.be.a("string");
            alice_reasonable_swap_href = swap_location;
        });

        it("[Alice] Should see Bob in her list of peers after sending a swap request to him", async () => {
            await sleep(1000);
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/peers"
            );

            res.should.have.status(200);
            res.body.peers.should.containSubset([
                {
                    id: bob_comit_node_address,
                },
            ]);
        });

        it("[Bob] Should see a new peer in his list of peers after receiving a swap request from Alice", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get("/peers");

            res.should.have.status(200);
            res.body.peers.should.have.length(1);
        });

        let alice_stingy_swap_href: string;
        it("[Alice] Should be able to make second swap request via HTTP api", async () => {
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
                        quantity: alpha_asset_stingy_quantity,
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

            const swap_location = res.header.location;
            swap_location.should.be.a("string");
            alice_stingy_swap_href = swap_location;
        });

        it("[Alice] Should still only see Bob in her list of peers after sending a second swap request to him", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/peers"
            );

            res.should.have.status(200);
            res.body.peers.should.containSubset([
                {
                    id: bob_comit_node_address,
                },
            ]);
        });

        it("[Bob] Should still only see one peer in his list of peers after receiving a second swap request from Alice", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get("/peers");

            res.should.have.status(200);
            res.body.peers.should.have.length(1);
        });

        it("[Alice] Shows the swaps as IN_PROGRESS in GET /swaps", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/swaps"
            );

            res.should.have.status(200);

            const swapEntities = res.body
                .entities as EmbeddedRepresentationSubEntity[];

            expect(swapEntities.map(entity => entity.properties))
                .to.each.have.property("status")
                .that.is.equal("IN_PROGRESS");
        });

        let bob_stingy_swap_href: string;
        let bob_reasonable_swap_href: string;

        it("[Bob] Shows the swaps as IN_PROGRESS in /swaps", async () => {
            const swapEntities = await bob
                .pollComitNodeUntil(
                    "/swaps",
                    body => body.entities.length === 2
                )
                .then(
                    body => body.entities as EmbeddedRepresentationSubEntity[]
                );

            expect(swapEntities.map(entity => entity.properties))
                .to.each.have.property("protocol")
                .that.is.equal("rfc003");
            expect(swapEntities.map(entity => entity.properties))
                .to.each.have.property("status")
                .that.is.equal("IN_PROGRESS");

            const stingy_swap = swapEntities.find(entity => {
                return (
                    parseInt(
                        entity.properties.parameters.alpha_asset.quantity,
                        10
                    ) === parseInt(alpha_asset_stingy_quantity, 10)
                );
            });
            const reasonable_swap = swapEntities.find(entity => {
                return (
                    parseInt(
                        entity.properties.parameters.alpha_asset.quantity,
                        10
                    ) === parseInt(alpha_asset_reasonable_quantity, 10)
                );
            });

            bob_stingy_swap_href = stingy_swap.links.find(link =>
                link.rel.includes("self")
            ).href;
            bob_reasonable_swap_href = reasonable_swap.links.find(link =>
                link.rel.includes("self")
            ).href;
        });

        it("[Bob] Has the RFC-003 parameters when GETing the swap", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get(
                bob_stingy_swap_href
            );

            res.should.have.status(200);

            const body = res.body as Entity;

            expect(body.properties).jsonSchema(swapPropertiesJsonSchema);
        });

        it("[Bob] Has the accept and decline actions when GETing the swap", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get(
                bob_stingy_swap_href
            );

            res.should.have.status(200);

            const body = res.body as Entity;

            expect(body.actions).containSubset([
                {
                    name: "accept",
                },
                {
                    name: "decline",
                },
            ]);
        });

        it("[Bob] Can execute a decline action", async () => {
            const bob = new Actor(
                "bob",
                global.config,
                global.project_root,
                null,
                {
                    reason: "BadRate",
                }
            );

            const res = await request(bob.comitNodeHttpApiUrl()).get(
                bob_stingy_swap_href
            );
            const body = res.body as Entity;

            const decline = body.actions.find(
                action => action.name === "decline"
            );
            const decline_res = await bob.doComitAction(decline);

            decline_res.should.have.status(200);
        });

        it("[Bob] Should be in the Rejected State after declining a swap request providing a reason", async function() {
            await bob.pollComitNodeUntil(
                bob_stingy_swap_href,
                entity =>
                    entity.properties.state.communication.status === "REJECTED"
            );
        });

        it("[Alice] Should be in the Rejected State after Bob declines a swap request providing a reason", async () => {
            await alice.pollComitNodeUntil(
                alice_stingy_swap_href,
                body =>
                    body.properties.state.communication.status === "REJECTED"
            );
        });

        it("[Bob] Can execute a decline action, without providing a reason", async () => {
            const bob = new Actor("bob", global.config, global.project_root);

            const res = await request(bob.comitNodeHttpApiUrl()).get(
                bob_reasonable_swap_href
            );
            const body = res.body as Entity;

            const decline = body.actions.find(
                action => action.name === "decline"
            );
            const decline_res = await bob.doComitAction(decline);

            decline_res.should.have.status(200);
        });

        it("[Bob] Should be in the Rejected State after declining a swap request without a reason", async () => {
            await bob.pollComitNodeUntil(
                bob_reasonable_swap_href,
                entity =>
                    entity.properties.state.communication.status === "REJECTED"
            );
        });

        it("[Alice] Should be in the Rejected State after Bob declines a swap request without a reason", async () => {
            await alice.pollComitNodeUntil(
                alice_reasonable_swap_href,
                entity =>
                    entity.properties.state.communication.status === "REJECTED"
            );
        });
    });

    run();
})();
