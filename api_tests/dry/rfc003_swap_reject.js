const chai = require("chai");
const Web3 = require("web3");
const actor = require("../lib/actor.js");
const should = chai.should();
chai.use(require("chai-http"));

const alpha_ledger_name = "Bitcoin";
const alpha_ledger_network = "regtest";

const beta_ledger_name = "Ethereum";
const beta_ledger_network = "regtest";

const alpha_asset_name = "Bitcoin";
const alpha_asset_reasonable_quantity = "100000000";
const alpha_asset_stingy_quantity = "100";

const beta_asset_name = "Ether";
const beta_asset_quantity = Web3.utils.toWei("10", "ether");

const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

const alice = actor.create("alice");
const bob = actor.create("bob");
const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const alice_comit_node_address = alice.config.comit.comit_listen;
const bob_comit_node_address = bob.config.comit.comit_listen;

let alice_reasonable_swap_href;
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
            alpha_ledger_refund_identity: null,
            beta_ledger_redeem_identity: alice_final_address,
            alpha_expiry: alpha_expiry,
            beta_expiry: beta_expiry,
            peer: bob_comit_node_address,
        })
        .then(res => {
            res.error.should.equal(false);
            res.should.have.status(201);
            swap_location = res.headers.location;
            swap_location.should.be.a("string");
            alice_reasonable_swap_href = swap_location;
        });
});

it("[Alice] Should see Bob in her list of peers after sending a swap request to him", async () => {
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

let alice_stingy_swap_href;
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
            alpha_ledger_refund_identity: null,
            beta_ledger_redeem_identity: alice_final_address,
            alpha_expiry: alpha_expiry,
            beta_expiry: beta_expiry,
            peer: bob_comit_node_address,
        })
        .then(res => {
            res.error.should.equal(false);
            res.should.have.status(201);
            swap_location = res.headers.location;
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

it("[Alice] Is able to GET the swap after POSTing it", async () => {
    await chai
        .request(alice.comit_node_url())
        .get(alice_reasonable_swap_href)
        .then(res => {
            res.should.have.status(200);

            let body = res.body;
            body.role.should.equal("Alice");
            body.state.should.equal("Start");
            let swap = body.swap;
            swap.should.be.a("object");
            swap.alpha_ledger.name.should.equal(alpha_ledger_name);
            swap.alpha_ledger.network.should.equal(alpha_ledger_network);
            swap.beta_ledger.name.should.equal(beta_ledger_name);
            swap.beta_ledger.network.should.equal(beta_ledger_network);
            swap.alpha_asset.name.should.equal(alpha_asset_name);
            swap.alpha_asset.quantity.should.equal(
                alpha_asset_reasonable_quantity
            );
            swap.beta_asset.name.should.equal(beta_asset_name);
            swap.beta_asset.quantity.should.equal(beta_asset_quantity);
            swap.alpha_expiry.should.equal(alpha_expiry);
            swap.beta_expiry.should.equal(beta_expiry);
        });
});

it("[Alice] Shows the swaps as Start in GET /swaps", async () => {
    await chai
        .request(alice.comit_node_url())
        .get("/swaps")
        .then(res => {
            res.should.have.status(200);
            let embedded = res.body._embedded;
            embedded.should.be.a("object");
            let swaps = embedded.swaps;
            let reasonable_swap_in_swaps = {
                _links: { self: { href: alice_reasonable_swap_href } },
                protocol: "rfc003",
                state: "Start",
            };
            let stingy_swap_in_swaps = {
                _links: { self: { href: alice_stingy_swap_href } },
                protocol: "rfc003",
                state: "Start",
            };
            swaps.should.have.deep.members([
                stingy_swap_in_swaps,
                reasonable_swap_in_swaps,
            ]);
        });
});

let bob_stingy_swap_href;
let bob_reasonable_swap_href;

it("[Bob] Shows the swaps as Start in /swaps", async () => {
    let res = await chai.request(bob.comit_node_url()).get("/swaps");
    let embedded = res.body._embedded;
    embedded.swaps.should.have.lengthOf(2);
    let swaps = embedded.swaps;

    for (let swap of swaps) {
        swap.protocol.should.equal("rfc003");
        swap.state.should.equal("Start");
    }

    let swap_1_link = swaps[0]._links.self;
    swap_1_link.should.be.a("object");
    let swap_1_href = swap_1_link.href;
    swap_1_href.should.be.a("string");
    let swap_1 = await chai.request(bob.comit_node_url()).get(swap_1_href);

    let swap_2_link = swaps[1]._links.self;
    swap_2_link.should.be.a("object");
    let swap_2_href = swap_2_link.href;
    swap_2_href.should.be.a("string");
    let swap_2 = await chai.request(bob.comit_node_url()).get(swap_2_href);

    if (
        swap_1.body.swap.alpha_asset.quantity ==
        parseInt(alpha_asset_stingy_quantity)
    ) {
        bob_stingy_swap_href = swap_1_href;
        bob_reasonable_swap_href = swap_2_href;
    } else {
        bob_stingy_swap_href = swap_2_href;
        bob_reasonable_swap_href = swap_1_href;
    }
});

let bob_decline_href_1;

it("[Bob] Has the accept and decline actions when GETing the swap", async () => {
    await chai
        .request(bob.comit_node_url())
        .get(bob_stingy_swap_href)
        .then(res => {
            res.should.have.status(200);

            let body = res.body;
            body.state.should.equal("Start");
            body.swap.should.be.a("object");
            let swap = body.swap;
            swap.alpha_ledger.name.should.equal(alpha_ledger_name);
            swap.alpha_ledger.network.should.equal(alpha_ledger_network);
            swap.beta_ledger.name.should.equal(beta_ledger_name);
            swap.beta_ledger.network.should.equal(beta_ledger_network);
            swap.alpha_asset.name.should.equal(alpha_asset_name);
            swap.alpha_asset.quantity.should.equal("100");
            swap.beta_asset.name.should.equal(beta_asset_name);
            swap.beta_asset.quantity.should.equal(beta_asset_quantity);
            swap.beta_asset.name.should.equal(beta_asset_name);
            swap.beta_asset.quantity.should.equal(beta_asset_quantity);
            swap.alpha_expiry.should.equal(alpha_expiry);
            swap.beta_expiry.should.equal(beta_expiry);

            let action_links = body._links;
            action_links.should.be.a("object");
            action_links.accept.should.be.a("object");
            action_links.accept.href.should.equal(
                bob_stingy_swap_href + "/accept"
            );

            action_links.decline.should.be.a("object");
            bob_decline_href_1 = action_links.decline.href;
            bob_decline_href_1.should.equal(bob_stingy_swap_href + "/decline");
        });
});

it("[Bob] Can execute a decline action providing a reason", async () => {
    let bob_response = {
        reason: "BadRate",
    };

    let decline_res = await chai
        .request(bob.comit_node_url())
        .post(bob_decline_href_1)
        .send(bob_response);

    decline_res.should.have.status(200);
});

it("[Bob] Should be in the Rejected State after declining a swap request providing a reason", async function() {
    await bob.poll_comit_node_until(chai, bob_stingy_swap_href, "Rejected");
});

it("[Alice] Should be in the Rejected State after Bob declines a swap request providing a reason", async () => {
    await alice.poll_comit_node_until(chai, alice_stingy_swap_href, "Rejected");
});

it("[Bob] Can execute a decline action, without providing a reason", async () => {
    let bob_decline_href_2;

    await chai
        .request(bob.comit_node_url())
        .get(bob_reasonable_swap_href)
        .then(res => {
            res.should.have.status(200);
            bob_decline_href_2 = res.body._links.decline.href;
            bob_decline_href_2.should.equal(
                bob_reasonable_swap_href + "/decline"
            );
        });

    let decline_res = await chai
        .request(bob.comit_node_url())
        .post(bob_decline_href_2)
        .send({});

    decline_res.should.have.status(200);
});

it("[Bob] Should be in the Rejected State after declining a swap request without a reason", async () => {
    await bob.poll_comit_node_until(chai, bob_reasonable_swap_href, "Rejected");
});

it("[Alice] Should be in the Rejected State after Bob declines a swap request without a reason", async () => {
    await alice.poll_comit_node_until(
        chai,
        alice_reasonable_swap_href,
        "Rejected"
    );
});
