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
        await chai.request(alice.comit_node_url())
            .post('/swaps/rfc003')
            .send({
                "alpha_ledger": {
                    "name": "Thomas' wallet",
                },
                "beta_ledger": {
                    "name": "Higher-Dimension" // This is the coffee place downstairs
                },
                "alpha_asset": {
                    "name": "AUD",
                    "quantity": "3.5"
                },
                "beta_asset": {
                    "name": "Espresso",
                    "double-shot": true
                },
                "alpha_ledger_refund_identity": "",
                "beta_ledger_redeem_identity": "",
                "alpha_ledger_lock_duration": 0
            }).then((res) => {
                res.should.have.status(400);
                res.body.title.should.equal("swap-not-supported");
            });
    });


    it("[Alice] Returns 400 bad request for malformed requests", async () => {
        await chai.request(alice.comit_node_url())
            .post('/swaps/rfc003')
            .send({
                "garbage": true
            }).then((res) => {
                res.should.have.status(400);
                res.body.title.should.equal("Bad Request");
            });
    });

    let swap_url_1;
    it("[Alice] Should be able to make first swap request via HTTP api", async () => {
        await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                alpha_ledger: {
                    name: alpha_ledger_name,
                    network: alpha_ledger_network
                },
                beta_ledger: {
                    name: beta_ledger_name
                },
                alpha_asset: {
                    name: alpha_asset_name,
                    quantity: alpha_asset_quantity
                },
                beta_asset: {
                    name: beta_asset_name,
                    quantity: beta_asset_quantity
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
                swap_url_1 = swap_location;
            });
    });

    let swap_url_2;
    it("[Alice] Should be able to make second swap request via HTTP api", async () => {
        await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                alpha_ledger: {
                    name: alpha_ledger_name,
                    network: alpha_ledger_network
                },
                beta_ledger: {
                    name: beta_ledger_name
                },
                alpha_asset: {
                    name: alpha_asset_name,
                    quantity: alpha_asset_quantity
                },
                beta_asset: {
                    name: beta_asset_name,
                    quantity: beta_asset_quantity
                },
                alpha_ledger_refund_identity: null,
                beta_ledger_redeem_identity: alice_final_address,
                alpha_ledger_lock_duration: 144
            })
            .then(res => {
                res.error.should.equal(false);
                res.should.have.status(201);
                swap_location = res.headers.location;
                swap_location.should.be.a("string");
                swap_url_2 = swap_location;
            });
    });

    it("[Alice] Is able to GET the swap after POSTing it", async () => {
        await chai
            .request(alice.comit_node_url())
            .get(swap_url_1)
            .then(res => {
                res.should.have.status(200);

                let body = res.body;
                body.role.should.equal("Alice");
                body.state.should.equal("Start");
                let swap = body.swap;
                swap.should.be.a("object");
                swap.alpha_ledger.name.should.equal(alpha_ledger_name);
                swap.alpha_ledger.network.should.equal(
                    alpha_ledger_network
                );
                swap.beta_ledger.name.should.equal(beta_ledger_name);
                swap.alpha_asset.name.should.equal(alpha_asset_name);
                swap.alpha_asset.quantity.should.equal(
                    alpha_asset_quantity
                );
                swap.beta_asset.name.should.equal(beta_asset_name);
                swap.beta_asset.quantity.should.equal(beta_asset_quantity);
                swap.alpha_lock_duration.type.should.equal("blocks");
                swap.alpha_lock_duration.value.should.equal(alpha_ledger_lock_duration);
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
                    links.self.href.should.be.oneOf([swap_url_1, swap_url_2]);
                }
            });
    });

    let swap_link_href;
    it("[Bob] Shows the swaps as Start in /swaps", async () => {
        let res = await chai.request(bob.comit_node_url()).get("/swaps");
        let embedded = res.body._embedded;
        embedded.swaps.should.have.lengthOf(2);
        let swaps = embedded.swaps;

        for (let swap of swaps) {
            swap.protocol.should.equal("rfc003");
            swap.state.should.equal("Start");
        }


        let swap = swaps[0];
        let swap_link = swap._links.self;
        swap_link.should.be.a("object");
        swap_link_href = swap_link.href;
        swap_link_href.should.be.a("string");
    });

    it("[Bob] Has the accept when GETing the swap", async () => {
        await chai
            .request(bob.comit_node_url())
            .get(swap_link_href)
            .then(res => {
                res.should.have.status(200);

                let body = res.body;
                body.state.should.equal("Start");
                body.swap.should.be.a("object");
                let swap = body.swap;
                swap.alpha_ledger.name.should.equal(alpha_ledger_name);
                swap.alpha_ledger.network.should.equal(
                    alpha_ledger_network
                );
                swap.beta_ledger.name.should.equal(beta_ledger_name);
                swap.alpha_asset.name.should.equal(alpha_asset_name);
                swap.alpha_asset.quantity.should.equal(
                    alpha_asset_quantity
                );
                swap.beta_asset.name.should.equal(beta_asset_name);
                swap.beta_asset.quantity.should.equal(beta_asset_quantity);
                swap.beta_asset.name.should.equal(beta_asset_name);
                swap.beta_asset.quantity.should.equal(beta_asset_quantity);
                swap.alpha_lock_duration.type.should.equal("blocks");
                swap.alpha_lock_duration.value.should.equal(alpha_ledger_lock_duration);

                let action_links = body._links;
                action_links.should.be.a("object");
                action_links.accept.should.be.a("object");
                action_links.accept.href.should.equal(
                    swap_link_href + "/accept"
                );

                action_links.decline.should.be.a("object");
                action_links.decline.href.should.equal(
                    swap_link_href + "/decline"
                );
            });
    });
});
