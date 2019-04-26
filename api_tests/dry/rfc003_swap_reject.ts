import { Actor } from "../lib/actor";
import * as util from "../lib/util";
import * as chai from "chai";
import { SwapResponse, SwapsResponse, Swap } from "../lib/comit";
import * as utils from "web3-utils";
import { HarnessGlobal } from "../lib/util";

import chaiHttp = require("chai-http");

chai.use(chaiHttp);
const should = chai.should();
declare var global: HarnessGlobal;

const alpha_ledger_name = "bitcoin";
const alpha_ledger_network = "regtest";

const beta_ledger_name = "ethereum";
const beta_ledger_network = "regtest";

const alpha_asset_name = "bitcoin";
const alpha_asset_reasonable_quantity = "100000000";
const alpha_asset_stingy_quantity = "100";

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
const bob_comit_node_address = bob.comitNodeConfig.comit.comit_listen;

// the `setTimeout` forces it to be added on the event loop
// This is needed because there is no async call in the test
// And hence it does not get run without this `setTimeout`
setTimeout(async function() {
    describe("SWAP request REJECTED", () => {
        let alice_reasonable_swap_href: string;
        it("[Alice] Should be able to make first swap request via HTTP api", async () => {
            await chai
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
                        quantity: alpha_asset_reasonable_quantity,
                    },
                    beta_asset: {
                        name: beta_asset_name,
                        quantity: beta_asset_quantity,
                    },
                    beta_ledger_redeem_identity: alice_final_address,
                    alpha_expiry: alpha_expiry,
                    beta_expiry: beta_expiry,
                    peer: bob_comit_node_address,
                })
                .then(res => {
                    res.error.should.equal(false);
                    res.should.have.status(201);
                    const swap_location = res.header.location;
                    swap_location.should.be.a("string");
                    alice_reasonable_swap_href = swap_location;
                });
        });

        it("[Alice] Should see Bob in her list of peers after sending a swap request to him", async () => {
            await util.sleep(1000);
            await chai
                .request(alice.comit_node_url())
                .get("/peers")
                .then(res => {
                    res.should.have.status(200);
                    res.body.peers.should.eql([bob_comit_node_address]);
                });
        });

        it("[Bob] Should see a new peer in his list of peers after receiving a swap request from Alice", async () => {
            await chai
                .request(bob.comit_node_url())
                .get("/peers")
                .then(res => {
                    res.should.have.status(200);
                    res.body.peers.should.have.length(1);
                });
        });

        let alice_stingy_swap_href: string;
        it("[Alice] Should be able to make second swap request via HTTP api", async () => {
            await chai
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
                        quantity: alpha_asset_stingy_quantity,
                    },
                    beta_asset: {
                        name: beta_asset_name,
                        quantity: beta_asset_quantity,
                    },
                    beta_ledger_redeem_identity: alice_final_address,
                    alpha_expiry: alpha_expiry,
                    beta_expiry: beta_expiry,
                    peer: bob_comit_node_address,
                })
                .then(res => {
                    res.error.should.equal(false);
                    res.should.have.status(201);
                    const swap_location = res.header.location;
                    swap_location.should.be.a("string");
                    alice_stingy_swap_href = swap_location;
                });
        });

        it("[Alice] Should still only see Bob in her list of peers after sending a second swap request to him", async () => {
            await chai
                .request(alice.comit_node_url())
                .get("/peers")
                .then(res => {
                    res.should.have.status(200);
                    res.body.peers.should.eql([bob_comit_node_address]);
                });
        });

        it("[Bob] Should still only see one peer in his list of peers after receiving a second swap request from Alice", async () => {
            await chai
                .request(bob.comit_node_url())
                .get("/peers")
                .then(res => {
                    res.should.have.status(200);
                    res.body.peers.should.have.length(1);
                });
        });

        it("[Alice] Shows the swaps as IN_PROGRESS in GET /swaps", async () => {
            await chai
                .request(alice.comit_node_url())
                .get("/swaps")
                .then(res => {
                    res.should.have.status(200);
                    let embedded = res.body._embedded;
                    embedded.should.be.a("object");
                    let swaps = embedded.swaps.map((swap: Swap) => ({
                        ...swap._links.self,
                        status: swap.status,
                    }));
                    let reasonable_swap = {
                        href: alice_reasonable_swap_href,
                        status: "IN_PROGRESS",
                    };
                    let stingy_swap = {
                        href: alice_stingy_swap_href,
                        status: "IN_PROGRESS",
                    };
                    swaps.should.have.deep.members([
                        stingy_swap,
                        reasonable_swap,
                    ]);
                });
        });

        let bob_stingy_swap_href: string;
        let bob_reasonable_swap_href: string;

        it("[Bob] Shows the swaps as Start in /swaps", async () => {
            let body = (await bob.pollComitNodeUntil(
                "/swaps",
                body => body._embedded.swaps.length === 2
            )) as SwapsResponse;

            let swaps = body._embedded.swaps;

            for (let swap of swaps) {
                swap.protocol.should.equal("rfc003");
                swap.status.should.equal("IN_PROGRESS");
                swap._links.accept.should.be.a("object");
                swap._links.decline.should.be.a("object");
            }

            let swap_1_link = swaps[0]._links.self;
            swap_1_link.should.be.a("object");
            let swap_1_href = swap_1_link.href;
            swap_1_href.should.be.a("string");
            let swap_1 = await chai
                .request(bob.comit_node_url())
                .get(swap_1_href);

            let swap_2_link = swaps[1]._links.self;
            swap_2_link.should.be.a("object");
            let swap_2_href = swap_2_link.href;
            swap_2_href.should.be.a("string");
            await chai.request(bob.comit_node_url()).get(swap_2_href);

            if (
                parseInt(swap_1.body.parameters.alpha_asset.quantity) ===
                parseInt(alpha_asset_stingy_quantity)
            ) {
                bob_stingy_swap_href = swap_1_href;
                bob_reasonable_swap_href = swap_2_href;
            } else {
                bob_stingy_swap_href = swap_2_href;
                bob_reasonable_swap_href = swap_1_href;
            }
        });

        let bob_decline_href_stingy: string;

        it("[Bob] Has the RFC-003 parameters when GETing the swap", async () => {
            await chai
                .request(bob.comit_node_url())
                .get(bob_stingy_swap_href)
                .then(res => {
                    res.should.have.status(200);

                    let body = res.body;
                    body.status.should.equal("IN_PROGRESS");
                    body.parameters.should.be.a("object");

                    let state = body.state;
                    state.should.be.a("object");

                    state.alpha_ledger.should.be.a("object");
                    state.beta_ledger.should.be.a("object");

                    let communication = state.communication;
                    communication.should.be.a("object");

                    communication.status.should.equal("SENT");

                    communication.alpha_expiry.should.be.a("number");
                    should.not.exist(communication.alpha_redeem_identity);
                    communication.alpha_refund_identity.should.be.a("string");

                    communication.beta_expiry.should.be.a("number");
                    communication.beta_redeem_identity.should.equal(
                        alice_final_address
                    );
                    should.not.exist(communication.beta_refund_identity);
                    communication.secret_hash.should.be.a("string");
                });
        });

        it("[Bob] Has the accept and decline actions when GETing the swap", async () => {
            await chai
                .request(bob.comit_node_url())
                .get(bob_stingy_swap_href)
                .then(res => {
                    res.should.have.status(200);

                    let action_links = res.body._links;
                    action_links.should.be.a("object");
                    action_links.accept.should.be.a("object");
                    action_links.accept.href.should.equal(
                        bob_stingy_swap_href + "/accept"
                    );

                    action_links.decline.should.be.a("object");
                    bob_decline_href_stingy = action_links.decline.href;
                    bob_decline_href_stingy.should.equal(
                        bob_stingy_swap_href + "/decline"
                    );
                });
        });

        it("[Bob] Can execute a decline action providing a reason", async () => {
            let bob_response = {
                reason: "BadRate",
            };

            let decline_res = await chai
                .request(bob.comit_node_url())
                .post(bob_decline_href_stingy)
                .send(bob_response);

            decline_res.should.have.status(200);
        });

        it("[Bob] Should be in the Rejected State after declining a swap request providing a reason", async function() {
            await bob.pollComitNodeUntil(
                bob_stingy_swap_href,
                (body: SwapResponse) =>
                    body.state.communication.status === "REJECTED"
            );
        });

        it("[Alice] Should be in the Rejected State after Bob declines a swap request providing a reason", async () => {
            await alice.pollComitNodeUntil(
                alice_stingy_swap_href,
                (body: SwapResponse) =>
                    body.state.communication.status === "REJECTED"
            );
        });

        it("[Bob] Can execute a decline action, without providing a reason", async () => {
            let bob_decline_href_2: string;

            let res = await chai
                .request(bob.comit_node_url())
                .get(bob_reasonable_swap_href);
            res.should.have.status(200);
            bob_decline_href_2 = res.body._links.decline.href;
            bob_decline_href_2.should.equal(
                bob_reasonable_swap_href + "/decline"
            );
            let decline_res = await chai
                .request(bob.comit_node_url())
                .post(bob_decline_href_2)
                .send({});

            decline_res.should.have.status(200);
        });

        it("[Bob] Should be in the Rejected State after declining a swap request without a reason", async () => {
            await bob.pollComitNodeUntil(
                bob_reasonable_swap_href,
                (body: SwapResponse) =>
                    body.state.communication.status === "REJECTED"
            );
        });

        it("[Alice] Should be in the Rejected State after Bob declines a swap request without a reason", async () => {
            await alice.pollComitNodeUntil(
                alice_reasonable_swap_href,
                (body: SwapResponse) =>
                    body.state.communication.status === "REJECTED"
            );
        });
    });

    run();
}, 0);
