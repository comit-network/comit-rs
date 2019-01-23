const chai = require("chai");
const Web3 = require("web3");
chai.use(require("chai-http"));
const bitcoin = require("../../../lib/bitcoin.js");
const actor = require("../../../lib/actor.js");
const ethutil = require("ethereumjs-util");
const ethereum = require("../../../lib/ethereum.js");
const should = chai.should();
const wallet = require("../../../lib/wallet.js");
const logger = global.harness.logger;

const bitcoin_rpc_client = bitcoin.create_client();

const toby_wallet = wallet.create();

const toby_initial_eth = "10";
const bob_initial_eth = "5";
const bob_initial_erc20 = BigInt(Web3.utils.toWei("10000", "ether"));

const alice = actor.create("alice", {});
const bob = actor.create("bob", {});

const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const bob_final_address =
    "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
const bob_comit_node_address = bob.config.comit.comit_listen;

const alpha_asset_amount = 100000000;
const beta_asset_amount = BigInt(Web3.utils.toWei("5000", "ether"));
const alpha_max_fee = 5000; // Max 5000 satoshis fee

describe("RFC003: Bitcoin for ERC20", () => {
    let token_contract_address;
    before(async function() {
        this.timeout(5000);
        await bitcoin.btc_activate_segwit();
        await toby_wallet.eth().fund(toby_initial_eth);
        await bob.wallet.eth().fund(bob_initial_eth);
        await alice.wallet.btc().fund(10);
        await alice.wallet.eth().fund(1);
        let receipt = await toby_wallet.eth().deploy_erc20_token_contract();
        token_contract_address = receipt.contractAddress;

        await bitcoin.btc_import_address(bob_final_address); // Watch only import
        await bitcoin.btc_generate();
    });

    it(bob_initial_erc20 + " tokens were minted to Bob", async function() {
        let bob_wallet_address = bob.wallet.eth().address();

        let receipt = await ethereum.mint_erc20_tokens(
            toby_wallet,
            token_contract_address,
            bob_wallet_address,
            bob_initial_erc20
        );

        receipt.status.should.equal(true);

        let erc20_balance = await ethereum.erc20_balance(
            bob_wallet_address,
            token_contract_address
        );

        (erc20_balance === bob_initial_erc20).should.equal(true);
    });

    let swap_location;
    let alice_swap_href;

    it("[Alice] Should be able to make a swap request via HTTP api", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .post("/swaps/rfc003")
            .send({
                alpha_ledger: {
                    name: "Bitcoin",
                    network: "regtest",
                },
                beta_ledger: {
                    name: "Ethereum",
                    network: "regtest",
                },
                alpha_asset: {
                    name: "Bitcoin",
                    quantity: alpha_asset_amount.toString(),
                },
                beta_asset: {
                    name: "ERC20",
                    quantity: beta_asset_amount.toString(),
                    token_contract: token_contract_address,
                },
                alpha_ledger_refund_identity: null,
                beta_ledger_redeem_identity: alice_final_address,
                alpha_ledger_lock_duration: 144,
                peer: bob_comit_node_address,
            });

        res.should.have.status(201);
        swap_location = res.headers.location;
        logger.info("Alice created a new swap at %s", swap_location);
        swap_location.should.be.a("string");
        alice_swap_href = swap_location;
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
            beta_ledger_refund_identity: bob.wallet.eth().address(),
            alpha_ledger_redeem_identity: null,
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

        logger.info(
            "Alice retrieved the following funding parameters",
            alice_funding_action
        );
    });

    it("[Alice] Can execute the funding action", async () => {
        alice_funding_action.payload.should.include.all.keys(
            "to",
            "amount",
            "network"
        );
        await alice.do(alice_funding_action);
    });

    it("[Alice] Should be in AlphaFunded state after executing the funding action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(chai, alice_swap_href, "AlphaFunded");
    });

    let bob_deploy_href;

    it("[Bob] Should be in AlphaFunded state after Alice executes the funding action", async function() {
        this.timeout(10000);
        let swap = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "AlphaFunded"
        );
        swap.should.have.property("_links");
        swap._links.should.have.property("deploy");
        bob_deploy_href = swap._links.deploy.href;
    });

    let bob_deploy_action;

    it("[Bob] Can get the deploy action from the ‘deploy’ link", async () => {
        let res = await chai.request(bob.comit_node_url()).get(bob_deploy_href);
        res.should.have.status(200);
        bob_deploy_action = res.body;

        logger.info(
            "Bob retrieved the following deployment parameters",
            bob_deploy_action
        );
    });

    it("[Bob] Can execute the deploy action", async () => {
        bob_deploy_action.payload.should.include.all.keys(
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        bob_deploy_action.payload.amount.should.equal("0");
        await bob.do(bob_deploy_action);
    });

    it("[Alice] Should be in AlphaFundedBetaDeployed state after Bob executes the deploy action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "AlphaFundedBetaDeployed"
        );
    });

    let bob_fund_href;

    it("[Bob] Should be in AlphaFundedBetaDeployed state after executing the deploy action", async function() {
        this.timeout(10000);
        let swap = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "AlphaFundedBetaDeployed"
        );
        let links = swap._links;
        links.should.have.property("fund");
        bob_fund_href = links.fund.href;
    });

    let bob_fund_action;

    it("[Bob] Can get the fund action from the ‘fund’ link", async () => {
        let res = await chai.request(bob.comit_node_url()).get(bob_fund_href);
        res.should.have.status(200);
        bob_fund_action = res.body;

        logger.info(
            "Bob retrieved the following funding parameters",
            bob_fund_action
        );
    });

    it("[Bob] Can execute the fund action", async () => {
        bob_fund_action.payload.should.include.all.keys(
            "contract_address",
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        let receipt = await bob.do(bob_fund_action);
        receipt.status.should.equal(true);
    });

    it("[Bob] Should be in BothFunded state after executing the funding action", async function() {
        this.timeout(100000000);
        await bob.poll_comit_node_until(chai, bob_swap_href, "BothFunded");
    });

    let alice_redeem_href;

    it("[Alice] Should be in BothFunded state after Bob executes the funding action", async function() {
        this.timeout(100000000);
        let swap = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "BothFunded"
        );
        swap.should.have.property("_links");
        swap._links.should.have.property("redeem");
        alice_redeem_href = swap._links.redeem.href;
    });

    let alice_redeem_action;

    it("[Alice] Can get the redeem action from the ‘redeem’ link", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_redeem_href);
        res.should.have.status(200);
        alice_redeem_action = res.body;

        logger.info(
            "Alice retrieved the following redeem parameters",
            alice_redeem_action
        );
    });

    let alice_erc20_balance_before;

    it("[Alice] Can execute the redeem action", async function() {
        alice_redeem_action.payload.should.include.all.keys(
            "contract_address",
            "data",
            "amount",
            "gas_limit",
            "network"
        );
        alice_erc20_balance_before = await ethereum.erc20_balance(
            alice_final_address,
            token_contract_address
        );
        await alice.do(alice_redeem_action);
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
        let alice_erc20_balance_after = await ethereum.erc20_balance(
            alice_final_address,
            token_contract_address
        );

        let alice_erc20_balance_expected =
            alice_erc20_balance_before + beta_asset_amount;
        alice_erc20_balance_after
            .toString()
            .should.equal(alice_erc20_balance_expected.toString());
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
            .get(
                bob_redeem_href +
                    "?address=" +
                    bob_final_address +
                    "&fee_per_byte=20"
            );
        res.should.have.status(200);
        bob_redeem_action = res.body;

        logger.info(
            "Bob retrieved the following redeem parameters",
            bob_redeem_action
        );
    });

    let bob_btc_balance_before;

    it("[Bob] Can execute the redeem action", async function() {
        bob_redeem_action.payload.should.include.all.keys("hex", "network");
        bob_btc_balance_before = await bitcoin.btc_balance(bob_final_address);
        await bob.do(bob_redeem_action);
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

    it("[Bob] Should have received the alpha asset after the redeem", async function() {
        let bob_btc_balance_after = await bitcoin.btc_balance(
            bob_final_address
        );
        const bob_btc_balance_expected =
            bob_btc_balance_before + alpha_asset_amount - alpha_max_fee;
        bob_btc_balance_after.should.be.at.least(bob_btc_balance_expected);
    });
});
