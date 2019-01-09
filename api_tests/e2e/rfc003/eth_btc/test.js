const chai = require("chai");
chai.use(require("chai-http"));
const actor = require("../../../lib/actor.js");
const bitcoin = require("../../../lib/bitcoin.js");
const ethutil = require("ethereumjs-util");
const should = chai.should();
const util = require("../../../lib/util.js");
const web3_conf = require("../../../lib/web3_conf.js");

const web3 = web3_conf.create();
const logger = util.logger();

let bitcoin_rpc_client = bitcoin.create_client()

const bob_initial_eth = "0.1";
const alice_initial_eth = "11";

const alice = actor.create("alice", {});
const bob = actor.create("bob", {});

const alice_final_address =
    "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
const bob_final_address = "0x03a329c0248369a73afac7f9381e02fb43d2ea72";
const bob_comit_node_address = bob.config.comit.comit_listen;

const alpha_asset_amount = new ethutil.BN(web3.utils.toWei("10", "ether"), 10);
const beta_asset_amount = 100000000;
const beta_max_fee = 5000; // Max 5000 satoshis fee

describe("RFC003: Ether for Bitcoin", () => {
    before(async function() {
        this.timeout(5000);
        await bitcoin.btc_activate_segwit();
        await alice.wallet.eth().fund(alice_initial_eth);
        await alice.wallet.btc().fund(0.1);
        await bob.wallet.eth().fund(bob_initial_eth);
        await bob.wallet.btc().fund(10);
        await bitcoin.btc_import_address(alice_final_address); // Watch only import
        await bitcoin.btc_import_address(
            bob.wallet.btc().identity().address
        ); // Watch only import
        await bitcoin.btc_import_address(
            alice.wallet.btc().identity().address
        ); // Watch only import
        await bitcoin.btc_generate();

        await bitcoin.log_btc_balance(
            "Before",
            "Alice",
            alice_final_address,
            "final"
        );
        await bitcoin.log_btc_balance(
            "Before",
            "Alice",
            alice.wallet.btc().identity().address,
            "wallet"
        );
        await ethereum.log_eth_balance(
            "Before",
            "Alice",
            alice.wallet.eth().address(),
            "wallet"
        );

        await ethereum.log_eth_balance(
            "Before",
            "Bob",
            bob_final_address,
            "final"
        );
        await bitcoin.log_btc_balance(
            "Before",
            "Bob",
            bob.wallet.btc().identity().address,
            "wallet"
        );
        await ethereum.log_eth_balance(
            "Before",
            "Bob",
            bob.wallet.eth().address(),
            "wallet"
        );
    });

    after(async function() {
        await bitcoin.log_btc_balance(
            "After",
            "Alice",
            alice_final_address,
            "final"
        );
        await bitcoin.log_btc_balance(
            "After",
            "Alice",
            alice.wallet.btc().identity().address,
            "wallet"
        );
        await ethereum.log_eth_balance(
            "After",
            "Alice",
            alice.wallet.eth().address(),
            "wallet"
        );

        await ethereum.log_eth_balance(
            "After",
            "Bob",
            bob_final_address,
            "final"
        );
        await bitcoin.log_btc_balance(
            "After",
            "Bob",
            bob.wallet.btc().identity().address,
            "wallet"
        );
        await ethereum.log_eth_balance(
            "After",
            "Bob",
            bob.wallet.eth().address(),
            "wallet"
        );
    });

    let swap_location;
    let alice_swap_href;

    it("[Alice] Should be able to make first swap request via HTTP api", async () => {
        await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                alpha_ledger: {
                    name: "Ethereum",
                },
                beta_ledger: {
                    name: "Bitcoin",
                    network: "regtest",
                },
                alpha_asset: {
                    name: "Ether",
                    quantity: alpha_asset_amount.toString(),
                },
                beta_asset: {
                    name: "Bitcoin",
                    quantity: beta_asset_amount.toString(),
                },
                alpha_ledger_refund_identity: alice.wallet.eth().address(),
                beta_ledger_redeem_identity: null,
                alpha_ledger_lock_duration: 144,
                peer: bob_comit_node_address,
            })
            .then(res => {
                res.should.have.status(201);
                swap_location = res.headers.location;
                logger.info("Alice created a new swap at %s", swap_location);
                swap_location.should.be.a("string");
                alice_swap_href = swap_location;
            });
    });

    it("[Alice] Should be in Start state after sending the swap request to Bob", async function() {
        await alice.poll_comit_node_until(chai, alice_swap_href, "Start");
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
        logger.info("Bob discovered a new swap at %s", bob_swap_href);
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
            beta_ledger_refund_identity: null,
            alpha_ledger_redeem_identity: bob_final_address,
            beta_ledger_lock_duration: 43200,
        };

        logger.info(
            "Bob is accepting the swap via %s with the following parameters",
            bob_accept_href,
            bob_response
        );

        let accept_res = await chai
            .request(bob.comit_node_url())
            .post(bob_accept_href)
            .send(bob_response);

        accept_res.should.have.status(200);
    });

    it("[Bob] Should be in the Accepted State after accepting", async () => {
        await bob.poll_comit_node_until(chai, bob_swap_href, "Accepted");
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

        logger.info(
            "Alice retrieved the following funding parameters",
            alice_funding_action
        );
    });

    it("[Alice] Can execute the funding action", async () => {
        alice_funding_action.should.include.all.keys(
            "data",
            "value",
            "gas_limit"
        );

        let result = await alice.wallet
            .eth()
            .deploy_contract(
                alice_funding_action.data,
                alice_funding_action.value,
                alice_funding_action.gas_limit
            );
    });

    it("[Alice] Should be in AlphaFunded state after executing the funding action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(chai, alice_swap_href, "AlphaFunded");
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

        logger.info(
            "Bob retrieved the following funding parameters",
            bob_funding_action
        );
    });

    it("[Bob] Can execute the funding action", async () => {
        bob_funding_action.should.include.all.keys("address", "value");
        await bob.wallet
            .btc()
            .send_btc_to_address(
                bob_funding_action.address,
                parseInt(bob_funding_action.value)
            );
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
        await bob.poll_comit_node_until(chai, bob_swap_href, "BothFunded");
    });

    let alice_redeem_action;

    it("[Alice] Can get the redeem action from the ‘redeem’ link", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(
                alice_redeem_href +
                    "?address=" +
                    alice_final_address +
                    "&fee_per_byte=20"
            );
        res.should.have.status(200);
        alice_redeem_action = res.body;

        logger.info(
            "Alice retrieved the following redeem parameters",
            alice_redeem_action
        );
    });

    let alice_btc_balance_before;

    it("[Alice] Can execute the redeem action", async function() {
        alice_redeem_action.should.include.all.keys("hex");
        alice_btc_balance_before = await bitcoin.btc_balance(
            alice_final_address
        );
        await bitcoin_rpc_client.sendRawTransaction(alice_redeem_action.hex);
        await bitcoin.btc_generate();
    });

    it("[Alice] Should be in AlphaFundedBetaRedeemed state after executing the redeem action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "AlphaFundedBetaRedeemed"
        );
    });

    it("[Alice] Should have received the beta asset after the redeem", async function() {
        let alice_btc_balance_after = await bitcoin.btc_balance(
            alice_final_address
        );

        const alice_btc_balance_expected =
            alice_btc_balance_before + beta_asset_amount - beta_max_fee;
        alice_btc_balance_after.should.be.at.least(alice_btc_balance_expected);
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
        let res = await chai.request(bob.comit_node_url()).get(bob_redeem_href);
        res.should.have.status(200);
        bob_redeem_action = res.body;

        logger.info(
            "Bob retrieved the following redeem parameters",
            bob_redeem_action
        );
    });

    let bob_eth_balance_before;

    it("[Bob] Can execute the redeem action", async function() {
        bob_redeem_action.should.include.all.keys(
            "to",
            "data",
            "gas_limit",
            "value"
        );
        bob_eth_balance_before = await ethereum.eth_balance(
            bob_final_address
        );
        await bob.wallet
            .eth()
            .send_eth_transaction_to(
                bob_redeem_action.to,
                bob_redeem_action.data,
                bob_redeem_action.value,
                bob_redeem_action.gas_limit
            );
    });

    it("[Bob] Should have received the alpha asset after the redeem", async function() {
        let bob_eth_balance_after = await ethereum.eth_balance(
            bob_final_address
        );

        let bob_eth_balance_expected = bob_eth_balance_before.add(
            alpha_asset_amount
        );
        bob_eth_balance_after
            .toString()
            .should.be.equal(bob_eth_balance_expected.toString());
    });

    it("[Alice] Should be in BothRedeemed state after Bob executes the redeem action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "BothRedeemed"
        );
    });

    it("[Bob] Should be in BothRedeemed state after executing the redeem action", async function() {
        this.timeout(10000);
        await bob.poll_comit_node_until(chai, bob_swap_href, "BothRedeemed");
    });
});
