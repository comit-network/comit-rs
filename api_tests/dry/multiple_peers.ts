import { Actor, TestConfig } from "../lib/actor";
import { EthConfig } from "../lib/ethereum";
import * as chai from "chai";
import { SwapResponse } from "../lib/comit";
import { BtcConfig } from "../lib/bitcoin";
import * as utils from "web3-utils";
import { HarnessGlobal } from "../lib/util";

import chaiHttp = require("chai-http");

declare var global: HarnessGlobal;

chai.use(chaiHttp);
const should = chai.should();

const alpha_ledger_name = "Bitcoin";
const alpha_ledger_network = "regtest";

const beta_ledger_name = "Ethereum";
const beta_ledger_network = "regtest";

const alpha_asset_name = "Bitcoin";
const alpha_asset_bob_quantity = "100000000";
const alpha_asset_charlie_quantity = "200000000";

const beta_asset_name = "Ether";
const beta_asset_bob_quantity = utils.toWei("10", "ether");
const beta_asset_charlie_quantity = utils.toWei("20", "ether");

const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

const alice = new Actor("alice", global.config, global.test_root, {
    ethConfig: global.ledgers_config.ethereum,
});
const bob = new Actor("bob", global.config, global.test_root, {
    ethConfig: global.ledgers_config.ethereum,
});
const charlie = new Actor("charlie", global.config, global.test_root, {
    ethConfig: global.ledgers_config.ethereum,
});

const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const alice_comit_node_address = alice.comitNodeConfig.comit.comit_listen;
const bob_comit_node_address = bob.comitNodeConfig.comit.comit_listen;
const charlie_comit_node_address = charlie.comitNodeConfig.comit.comit_listen;

let alice_swap_with_charlie_href: string;
async function test() {
    describe("SWAP requests to multiple peers", () => {
        it("[Alice] Should be able to send a swap request to Bob", async () => {
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
        });

        it("[Alice] Should be able to send a swap request to Charlie", async () => {
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
                (body: SwapResponse) =>
                    body.status === "IN_PROGRESS" &&
                    body.state.communication.status === "SENT"
            );
        });

        it("[Charlie] Shows the Swap as IN_PROGRESS in /swaps", async () => {
            let body = (await charlie.pollComitNodeUntil(
                "/swaps",
                (body: SwapResponse) => body._embedded.swaps.length > 0
            )) as SwapResponse;

            let swap_embedded = body._embedded.swaps[0];
            swap_embedded.protocol.should.equal("rfc003");
            swap_embedded.status.should.equal("IN_PROGRESS");
            let swap_link = swap_embedded._links;
            swap_link.should.be.a("object");
            let swap_href = swap_link.self.href;
            swap_href.should.be.a("string");
        });

        it("[Alice] Should see both Bob and Charlie in her list of peers after sending a swap request to both of them", async () => {
            let res = await chai.request(alice.comit_node_url()).get("/peers");
            res.should.have.status(200);
            res.body.peers.should.have.deep.members([
                charlie_comit_node_address,
                bob_comit_node_address,
            ]);
        });
    });

    run();
}

setTimeout(test, 0);
