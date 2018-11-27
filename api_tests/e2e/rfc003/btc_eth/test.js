const chai = require("chai");
const BigNumber = require("bignumber.js");
chai.use(require("chai-http"));
const Toml = require("toml");
const test_lib = require("../../../test_lib.js");
const should = chai.should();
const EthereumTx = require("ethereumjs-tx");
const assert = require("assert");
const fs = require("fs");
const ethutil = require("ethereumjs-util");

const web3 = test_lib.web3();

const bob_initial_eth = "11";
const bob_config = Toml.parse(
    fs.readFileSync(process.env.BOB_CONFIG_FILE, "utf8")
);

const alice_initial_eth = "0.1";
const alice = test_lib.comit_conf("alice", {});

const bob = test_lib.comit_conf("bob", {});

const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const beta_asset = new BigNumber(web3.utils.toWei("10", "ether"));
const bitcoin_rpc_client = test_lib.bitcoin_rpc_client();

describe("RFC003 Bitcoin for Ether", () => {
    before(async function() {
        this.timeout(5000);
        await test_lib.fund_eth(20);
        await test_lib.give_eth_to(bob.wallet.eth_address(), bob_initial_eth);
        await test_lib.give_eth_to(
            alice.wallet.eth_address(),
            alice_initial_eth
        );
    });

    let swap_location;
    let alice_swap_href;
    it("[Alice] Should be able to make first swap request via HTTP api", async () => {
        return chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                alpha_ledger: {
                    name: "Bitcoin",
                    network: "regtest"
                },
                beta_ledger: {
                    name: "Ethereum"
                },
                alpha_asset: {
                    name: "Bitcoin",
                    quantity: "100000000"
                },
                beta_asset: {
                    name: "Ether",
                    quantity: beta_asset.toString()
                },
                alpha_ledger_refund_identity:
                    "ac2db2f2615c81b83Fe9366450799b4992931575",
                beta_ledger_success_identity: alice_final_address,
                alpha_ledger_lock_duration: 144
            })
            .then(res => {
                res.should.have.status(201);
                swap_location = res.headers.location;
                swap_location.should.be.a("string");
                alice_swap_href = swap_location;
            });
    });

    let swap_link_href;

    it("[Bob] Shows the Swap as Start in /swaps", async () => {
        let res = await chai.request(bob.comit_node_url()).get("/swaps");

        let embedded = res.body._embedded;
        let swap_embedded = embedded.swaps[0];
        swap_embedded.protocol.should.equal("rfc003");
        swap_embedded.state.should.equal("Start");
        let swap_link = swap_embedded._links;
        swap_link.should.be.a("object");
        swap_link_href = swap_link.self.href;
        swap_link_href.should.be.a("string");
    });

    it("[Bob] Can execute the accept action", async () => {
        let res = await chai.request(bob.comit_node_url()).get(swap_link_href);
        res.should.have.status(200);
        res.body.state.should.equal("Start");
        res.body._links.accept.href.should.be.a("string");
        let accept_href = res.body._links.accept.href;
        let bob_response = {
            beta_ledger_refund_identity: bob.wallet.eth_address(),
            alpha_ledger_success_identity: bob.wallet
                .btc_address()
                .hash.toString("hex"),
            beta_ledger_lock_duration: 43200
        };

        let accept_res = await chai
            .request(bob.comit_node_url())
            .post(accept_href)
            .send(bob_response);

        accept_res.should.have.status(200);
    });

    it("[Bob] Should be in the Accepted State after accepting", async () => {
        let res = await chai.request(bob.comit_node_url()).get(swap_link_href);
        res.should.have.status(200);
        res.body.state.should.equal("Accepted");
    });

    let alice_funding_href;

    it("[Alice] Can get the HTLC fund action", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_swap_href);
        res.should.have.status(200);
        res.body.state.should.equal("Accepted");
        let links = res.body._links;
        links.should.have.property("fund");
        alice_funding_href = links.fund.href;
    });

    it("[Alice] Can get the funding instructions from the ‘fund’ link", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_funding_href);
        res.should.have.status(200);
    });

    // let alice_funding_required;

    // it("The request should eventually be accepted by Bob", function (done) {
    //     this.timeout(10000);
    //     alice.poll_comit_node_until(chai, swap_location, "accepted").then((status) => {
    //         alice_funding_required = status.funding_required;
    //         done();
    //     });
    // });

    // it("Alice should be able to manually fund the bitcoin HTLC", async function () {
    //     this.slow(500);
    //     return alice.wallet.send_btc_to_p2wsh_address(alice_funding_required, 100000000);
    // });

    // let redeem_details;

    // it("Bob should eventually deploy the Ethereum HTLC and Alice should see it", function (done) {
    //     this.slow(7000);
    //     this.timeout(10000);
    //     alice.poll_comit_node_until(chai, swap_location, "redeemable").then((status) => {
    //         redeem_details = status;
    //         done();
    //     });
    // });

    // it("Alice should be able to redeem Ether", async function () {
    //     this.slow(6000);
    //     this.timeout(10000);
    //     await test_lib.sleep(2000);
    //     let old_balance = new BigNumber(await web3.eth.getBalance(alice_final_address));
    //     await alice.wallet.send_eth_transaction_to(redeem_details.contract_address, "0x" + redeem_details.data);
    //     await test_lib.sleep(2000);
    //     let new_balance = new BigNumber(await web3.eth.getBalance(alice_final_address));
    //     let diff = new_balance.minus(old_balance);
    //     diff.toString().should.equal(beta_asset.toString());
    // });
});
