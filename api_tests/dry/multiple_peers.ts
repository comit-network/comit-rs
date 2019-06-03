import { Actor } from "../lib/actor";
import { toWei } from "web3-utils";
import { HarnessGlobal } from "../lib/util";
import { expect, request } from "chai";
import "../lib/setupChai";
import "chai/register-should";
import { EmbeddedRepresentationSubEntity } from "../gen/siren";

declare var global: HarnessGlobal;

(async () => {
    const alpha_ledger_name = "bitcoin";
    const alpha_ledger_network = "regtest";

    const beta_ledger_name = "ethereum";
    const beta_ledger_network = "regtest";

    const alpha_asset_name = "bitcoin";
    const alpha_asset_bob_quantity = "100000000";
    const alpha_asset_charlie_quantity = "200000000";

    const beta_asset_name = "ether";
    const beta_asset_bob_quantity = toWei("10", "ether");
    const beta_asset_charlie_quantity = toWei("20", "ether");

    const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    const alice = new Actor("alice", global.config, global.project_root);
    const bob = new Actor("bob", global.config, global.project_root);
    const charlie = new Actor("charlie", global.config, global.project_root);

    const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const alice_comit_node_address = await alice.peerId();
    const bob_comit_node_address = await bob.peerId();
    const charlie_comit_node_address = await charlie.peerId();

    let alice_swap_with_charlie_href: string;
    let alice_swap_with_bob_href: string;

    describe("SWAP requests to multiple peers", () => {
        it("[Alice] Should be able to send a swap request to Bob", async () => {
            let res = await request(alice.comit_node_url())
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
                        quantity: alpha_asset_bob_quantity,
                    },
                    beta_asset: {
                        name: beta_asset_name,
                        quantity: beta_asset_bob_quantity,
                    },
                    beta_ledger_redeem_identity: alice_final_address,
                    alpha_expiry: alpha_expiry,
                    beta_expiry: beta_expiry,
                    peer: bob_comit_node_address,
                });

            res.error.should.equal(false);
            res.should.have.status(201);

            const swap_location = res.header.location;
            swap_location.should.be.a("string");
            alice_swap_with_bob_href = swap_location;
        });

        it("[Alice] Should be able to send a swap request to Charlie", async () => {
            let res = await request(alice.comit_node_url())
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
                        quantity: alpha_asset_charlie_quantity,
                    },
                    beta_asset: {
                        name: beta_asset_name,
                        quantity: beta_asset_charlie_quantity,
                    },
                    beta_ledger_redeem_identity: alice_final_address,
                    alpha_expiry: alpha_expiry,
                    beta_expiry: beta_expiry,
                    peer: charlie_comit_node_address,
                });

            res.error.should.equal(false);
            res.should.have.status(201);

            const swap_location = res.header.location;
            swap_location.should.be.a("string");
            alice_swap_with_charlie_href = swap_location;
        });

        it("[Alice] Should be IN_PROGRESS and SENT after sending the swap request to Charlie", async function() {
            return alice.pollComitNodeUntil(
                alice_swap_with_charlie_href,
                body =>
                    body.properties.status === "IN_PROGRESS" &&
                    body.properties.state.communication.status === "SENT"
            );
        });

        it("[Alice] Should be able to see Bob's peer-id after sending the swap request to Bob", async function() {
            return alice.pollComitNodeUntil(
                alice_swap_with_bob_href,
                body => body.properties.counterparty === bob_comit_node_address
            );
        });

        it("[Alice] Should be able to see Charlie's peer-id after sending the swap request to Charlie", async function() {
            return alice.pollComitNodeUntil(
                alice_swap_with_charlie_href,
                body =>
                    body.properties.counterparty === charlie_comit_node_address
            );
        });

        it("[Charlie] Shows the Swap as IN_PROGRESS in /swaps", async () => {
            let swapEntity = await charlie
                .pollComitNodeUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            expect(swapEntity.properties).to.have.property(
                "protocol",
                "rfc003"
            );
            expect(swapEntity.properties).to.have.property(
                "status",
                "IN_PROGRESS"
            );
            expect(swapEntity.links).containSubset([
                {
                    rel: (expectedValue: string[]) =>
                        expectedValue.includes("self"),
                },
            ]);
        });

        it("[Charlie] Should be able to see Alice's peer-id after receiving the request", async function() {
            let swapEntity = await charlie
                .pollComitNodeUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            expect(swapEntity.properties).to.have.property(
                "counterparty",
                alice_comit_node_address
            );
        });

        it("[Alice] Should see both Bob and Charlie in her list of peers after sending a swap request to both of them", async () => {
            let res = await request(alice.comit_node_url()).get("/peers");

            res.should.have.status(200);
            res.body.peers.should.have.containSubset([
                {
                    id: bob_comit_node_address,
                },
                {
                    id: charlie_comit_node_address,
                },
            ]);
        });
    });

    run();
})();
