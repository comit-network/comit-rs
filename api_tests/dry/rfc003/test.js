const chai = require("chai");
const test_lib = require("../../test_lib.js");
const should = chai.should();
chai.use(require("chai-http"));
const web3 = test_lib.web3();
const BigNumber = require("bignumber.js");

const alpha_ledger_name = "Bitcoin";
const alpha_ledger_network = "regtest";

const beta_ledger_name = "Ethereum";

const alpha_asset_name = "Bitcoin";
const alpha_asset_quantity = "100000000";

const beta_asset_name = "Ether";
const beta_asset_quantity = new BigNumber(
    web3.utils.toWei("10", "ether")
).toString();

const alpha_ledger_lock_duration = 144;

const alice = test_lib.comit_conf("alice", {});
const bob = test_lib.comit_conf("bob", {});

const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

describe("RFC003 HTTP API", () => {
    it("[Alice] Returns 404 when you try and GET a non-existent swap", async () => {
        await chai
            .request(alice.comit_node_url())
            .get("/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead")
            .then(res => {
                res.should.have.status(404);
            });
    });

    it("Returns a 404 for an action on a non-existent swap", async () => {
        return chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead/accept")
            .send({})
            .then(res => {
                res.should.have.status(404);
            });
    });

    it("Returns an empty list when calling GET /swaps when there are no swaps", async () => {
        await chai
            .request(alice.comit_node_url())
            .get("/swaps")
            .then(res => {
                let swaps = res.body._embedded.swaps;
                swaps.should.be.an("array");
                swaps.should.have.lengthOf(0);
            });
    });

    it("[Alice] Returns 400 swap-not-supported for an unsupported combination of parameters", async () => {
        await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                alpha_ledger: {
                    name: "Thomas' wallet",
                },
                beta_ledger: {
                    name: "Higher-Dimension", // This is the coffee place downstairs
                },
                alpha_asset: {
                    name: "AUD",
                    quantity: "3.5",
                },
                beta_asset: {
                    name: "Espresso",
                    "double-shot": true,
                },
                alpha_ledger_refund_identity: "",
                beta_ledger_redeem_identity: "",
                alpha_ledger_lock_duration: 0,
            })
            .then(res => {
                res.should.have.status(400);
                res.body.title.should.equal("swap-not-supported");
            });
    });

    it("[Alice] Returns 400 bad request for malformed requests", async () => {
        await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                garbage: true,
            })
            .then(res => {
                res.should.have.status(400);
                res.body.title.should.equal("Bad Request");
            });
    });

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
                },
                alpha_asset: {
                    name: alpha_asset_name,
                    quantity: alpha_asset_quantity,
                },
                beta_asset: {
                    name: beta_asset_name,
                    quantity: beta_asset_quantity,
                },
                alpha_ledger_refund_identity: null,
                beta_ledger_redeem_identity: alice_final_address,
                alpha_ledger_lock_duration: alpha_ledger_lock_duration,
            })
            .then(res => {
                res.error.should.equal(false);
                res.should.have.status(201);
                swap_location = res.headers.location;
                swap_location.should.be.a("string");
                alice_reasonable_swap_href = swap_location;
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
                },
                alpha_asset: {
                    name: alpha_asset_name,
                    quantity: alpha_asset_quantity,
                },
                beta_asset: {
                    name: beta_asset_name,
                    quantity: beta_asset_quantity,
                },
                alpha_ledger_refund_identity: null,
                beta_ledger_redeem_identity: alice_final_address,
                alpha_ledger_lock_duration: alpha_ledger_lock_duration
            })
            .then(res => {
                res.error.should.equal(false);
                res.should.have.status(201);
                swap_location = res.headers.location;
                swap_location.should.be.a("string");
                alice_stingy_swap_href = swap_location;
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
                swap.alpha_asset.name.should.equal(alpha_asset_name);
                swap.alpha_asset.quantity.should.equal(alpha_asset_quantity);
                swap.beta_asset.name.should.equal(beta_asset_name);
                swap.beta_asset.quantity.should.equal(beta_asset_quantity);
                swap.alpha_lock_duration.type.should.equal("blocks");
                swap.alpha_lock_duration.value.should.equal(
                    alpha_ledger_lock_duration
                );
            });
    });

    it("[Alice] Shows the swaps in GET /swaps", async () => {
        await chai
            .request(alice.comit_node_url())
            .get("/swaps")
            .then(res => {
                res.should.have.status(200);
                let embedded = res.body._embedded;
                embedded.should.be.a("object");
                embedded.swaps.should.have.lengthOf(2);
                let swaps = embedded.swaps;
                for (swap of swaps) {
                    swap.protocol.should.equal("rfc003");
                    swap.state.should.equal("Start");
                    let links = swap._links;
                    links.self.href.should.be.oneOf([alice_reasonable_swap_href, alice_stingy_swap_href]);
                }
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

        if (swap_1.body.swap.alpha_asset.quantity == 100) {
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
                swap.alpha_asset.name.should.equal(alpha_asset_name);
                swap.alpha_asset.quantity.should.equal(
                    "100"
                );
                swap.beta_asset.name.should.equal(beta_asset_name);
                swap.beta_asset.quantity.should.equal(beta_asset_quantity);
                swap.beta_asset.name.should.equal(beta_asset_name);
                swap.beta_asset.quantity.should.equal(beta_asset_quantity);
                swap.alpha_lock_duration.type.should.equal("blocks");
                swap.alpha_lock_duration.value.should.equal(
                    alpha_ledger_lock_duration
                );

                let action_links = body._links;
                action_links.should.be.a("object");
                action_links.accept.should.be.a("object");
                action_links.accept.href.should.equal(
                    bob_stingy_swap_href + "/accept"
                );

                action_links.decline.should.be.a("object");
                bob_decline_href_1 = action_links.decline.href;
                bob_decline_href_1.should.equal(
                    bob_stingy_swap_href + "/decline"
                );
            });
    });

    it("[Bob] Can execute a decline action providing a reason", async () => {
        let bob_response = {
            reason: "You're very greedy, Alice"
        };

        let decline_res = await chai
            .request(bob.comit_node_url())
            .post(bob_decline_href_1)
            .send(bob_response);

        decline_res.should.have.status(200);
    });

    it("[Bob] Should be in the Rejected State after declining a swap request providing a reason", async () => {
        let res = await chai.request(bob.comit_node_url()).get(bob_stingy_swap_href);
        res.should.have.status(200);
        res.body.state.should.equal("Rejected");
    });

    it("[Alice] Should be in the Rejected State after Bob declines a swap request providing a reason", async () => {
        let res = await chai.request(alice.comit_node_url()).get(alice_stingy_swap_href);
        res.should.have.status(200);
        res.body.state.should.equal("Rejected");
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
        let res = await chai.request(bob.comit_node_url()).get(bob_reasonable_swap_href);
        res.should.have.status(200);
        res.body.state.should.equal("Rejected");
    });

    it("[Alice] Should be in the Rejected State after Bob declines a swap request without a reason", async () => {
        let res = await chai.request(alice.comit_node_url()).get(alice_reasonable_swap_href);
        res.should.have.status(200);
        res.body.state.should.equal("Rejected");
    });

});
