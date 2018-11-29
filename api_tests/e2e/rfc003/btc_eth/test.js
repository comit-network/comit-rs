const chai = require("chai");
chai.use(require("chai-http"));
const test_lib = require("../../../test_lib.js");
const should = chai.should();
const ethutil = require("ethereumjs-util");

const web3 = test_lib.web3();

const bob_initial_eth = "11";
const alice_initial_eth = "0.1";

const alice = test_lib.comit_conf("alice", {});
const bob = test_lib.comit_conf("bob", {});

const alice_final_address = "0x00a329c0648769a73afac7f9381e02fb43dbea72";
const bob_final_address = "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";

const alpha_asset = "100000000";
const beta_asset = new ethutil.BN(web3.utils.toWei("10", "ether"), 10);

describe("RFC003 Bitcoin for Ether", () => {
    before(async function() {
        this.timeout(5000);
        await bob.wallet.fund_eth(bob_initial_eth);
        await alice.wallet.fund_eth(alice_initial_eth);
        await alice.wallet.fund_btc(10);
    });

    let swap_location;
    let alice_swap_href;
    it("[Alice] Should be able to make first swap request via HTTP api", async () => {
        await chai
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
                    quantity: alpha_asset
                },
                beta_asset: {
                    name: "Ether",
                    quantity: beta_asset.toString()
                },
                alpha_ledger_refund_identity: null,
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

    let bob_swap_href;

    it("[Bob] Shows the Swap as Start in /swaps", async () => {
        let res = await chai.request(bob.comit_node_url()).get("/swaps");

        let embedded = res.body._embedded;
        let swap_embedded = embedded.swaps[0];
        swap_embedded.protocol.should.equal("rfc003");
        swap_embedded.state.should.equal("Start");
        let swap_link = swap_embedded._links;
        swap_link.should.be.a("object");
        bob_swap_href = swap_link.self.href;
        bob_swap_href.should.be.a("string");
    });

    let bob_accept_href;
    it("[Bob] Can get the accept action", async () => {
        let res = await chai.request(bob.comit_node_url()).get(bob_swap_href);
        res.should.have.status(200);
        res.body.state.should.equal("Start");
        res.body._links.accept.href.should.be.a("string");
        bob_accept_href = res.body._links.accept.href;
    });

    it("[Bob] Can execute the accept action", async () => {
        let bob_response = {
            beta_ledger_refund_identity: bob.wallet.eth_address(),
            alpha_ledger_success_identity: null,
            beta_ledger_lock_duration: 43200
        };

        let accept_res = await chai
            .request(bob.comit_node_url())
            .post(bob_accept_href)
            .send(bob_response);

        accept_res.should.have.status(200);
    });

    it("[Bob] Should be in the Accepted State after accepting", async () => {
        let res = await chai.request(bob.comit_node_url()).get(bob_swap_href);
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

    let alice_funding_action;

    it("[Alice] Can get the funding action from the ‘fund’ link", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_funding_href);
        res.should.have.status(200);
        alice_funding_action = res.body;
    });

    it("[Alice] Can execute the funding action", async () => {
        alice_funding_action.should.include.all.keys("address", "value");
        await alice.wallet.send_btc_to_address(
            alice_funding_action.address,
            parseInt(alice_funding_action.value)
        );
    });

    it("[Alice] Should be in AlphaFunded state after executing the funding action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "AlphaFunded"
        );
    });

    let bob_funding_href;

    it("[Bob] Should be in AlphaFunded state after Alice executes the funding action", async function() {
        this.timeout(10000);
        let swap = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "AlphaFunded"
        );
        swap.should.have.property("_links");
        swap._links.should.have.property("fund");
        bob_funding_href = swap._links.fund.href;
    });

    let bob_funding_action;

    it("[Bob] Can get the funding action from the ‘fund’ link", async () => {
        let res = await chai
            .request(bob.comit_node_url())
            .get(bob_funding_href);
        res.should.have.status(200);
        bob_funding_action = res.body;
    });

    it("[Bob] Can execute the funding action", async () => {
        bob_funding_action.should.include.all.keys("data", "gas_limit", "value");
        await bob.wallet.deploy_eth_contract(bob_funding_action.data, new ethutil.BN(bob_funding_action.value, 10));
    });

    let alice_redeem_href;

    it("[Alice] Should be in BothFunded state after Bob executes the funding action", async function() {
        this.timeout(10000);
        let swap = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "BothFunded"
        );
        swap.should.have.property("_links");
        swap._links.should.have.property("redeem");
        alice_redeem_href = swap._links.redeem.href;
    });

    it("[Bob] Should be in BothFunded state after executing the funding action", async function() {
        this.timeout(10000);
        await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "BothFunded"
        );
    });

    let alice_redeem_action;

    it("[Alice] Can get the redeem action from the ‘redeem’ link", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_redeem_href);
        res.should.have.status(200);
        alice_redeem_action = res.body;
    });

    let alice_eth_balance_before;

    it("[Alice] Can execute the redeem action", async function () {
        alice_redeem_action.should.include.all.keys("to", "data", "gas_limit", "value");
        alice_eth_balance_before = await test_lib.eth_balance(alice_final_address);
        await alice.wallet.send_eth_transaction_to(
            alice_redeem_action.to,
            alice_redeem_action.data,
            alice_redeem_action.value,
            alice_redeem_action.gas_limit)
    });

    it("[Alice] Should be in AlphaFundedBetaRedeemed state after executing the redeem action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "AlphaFundedBetaRedeemed"
        );
        let alice_eth_balance_after = await test_lib.eth_balance(alice_final_address);
        let alice_eth_balance_expected_balance = new ethutil.BN(alice_eth_balance_before, 10).add(beta_asset).toString();
        alice_eth_balance_after.should.equal(alice_eth_balance_expected_balance);
    });

    let bob_redeem_href;

    it("[Bob] Should be in AlphaFundedBetaRedeemed state after Alice executes the redeem action", async function() {
        this.timeout(10000);
        let swap = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "AlphaFundedBetaRedeemed"
        );
        swap.should.have.property("_links");
        swap._links.should.have.property("redeem");
        bob_redeem_href = swap._links.redeem.href;
    });

    let bob_redeem_action;

    it("[Bob] Can get the redeem action from the ‘redeem’ link", async () => {
        let res = await chai
            .request(bob.comit_node_url())
            .get(bob_redeem_href + "?address=" + bob_final_address + "&fee_per_byte=20");
        res.should.have.status(200);
        bob_redeem_action = res.body;
    });

    let bob_btc_balance_before;

    it("[Bob] Can execute the redeem action", async function () {
        bob_redeem_action.should.include.all.keys("hex");
        bob_btc_balance_before = await test_lib.btc_balance(bob_final_address);
        await bob.wallet.send_raw_tx(bob_redeem_action.hex)
    });

    it("[Alice] Should be in BothRedeemed state after Bob executes the redeem action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "BothRedeemed"
        );

        let bob_btc_balance_after = await test_lib.btc_balance(bob_final_address);
        bob_btc_balance_after.should.equal(bob_btc_balance_before + alpha_asset);
    });

    it("[Bob] Should be in BothRedeemed state after executing the redeem action", async function() {
        this.timeout(10000);
        await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "BothRedeemed"
        );
    });
});
