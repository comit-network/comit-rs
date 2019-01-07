const chai = require("chai");
chai.use(require("chai-http"));
const test_lib = require("../../../test_lib.js");
const should = chai.should();
const ethutil = require("ethereumjs-util");

const web3 = test_lib.web3();
const logger = test_lib.logger();

const toby_wallet = test_lib.wallet_conf();

const toby_initial_eth = "10";
const bob_initial_eth = "5";
const bob_initial_erc20 = web3.utils.toWei("10000", "ether");

const alice = test_lib.comit_conf("alice", {});
const bob = test_lib.comit_conf("bob", {});

const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const bob_final_address =
    "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";

const alpha_asset_amount = 100000000;
const beta_asset_amount = new ethutil.BN(web3.utils.toWei("5000", "ether"), 10);
const alpha_max_fee = 5000; // Max 5000 satoshis fee

describe("RFC003: Bitcoin for ERC20", () => {
    let token_contract_address;
    before(async function() {
        this.timeout(5000);
        await test_lib.btc_activate_segwit();
        await toby_wallet.fund_eth(toby_initial_eth);
        await bob.wallet.fund_eth(bob_initial_eth);
        await alice.wallet.fund_btc(10);
        await alice.wallet.fund_eth(1);
        let receipt = await toby_wallet.deploy_erc20_token_contract();
        token_contract_address = receipt.contractAddress;

        await test_lib.btc_import_address(bob_final_address); // Watch only import
        await test_lib.btc_generate();
    });

    it(bob_initial_erc20 + " tokens were minted to Bob", async function() {
        let bob_wallet_address = bob.wallet.eth_address();

        let receipt = await test_lib.mint_erc20_tokens(
            toby_wallet,
            token_contract_address,
            bob_wallet_address,
            bob_initial_erc20
        );

        receipt.status.should.equal(true);

        let erc20_balance = await test_lib.erc20_balance(
            bob_wallet_address,
            token_contract_address
        );
        erc20_balance.toString().should.equal(bob_initial_erc20);
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
            beta_ledger_refund_identity: bob.wallet.eth_address(),
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
        alice_funding_action.should.include.all.keys("address", "value");

        await alice.wallet.send_btc_to_address(
            alice_funding_action.address,
            parseInt(alice_funding_action.value)
        );
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
        bob_deploy_action.should.include.all.keys("data", "gas_limit", "value");
        bob_deploy_action.value.should.equal("0");
        await bob.wallet.deploy_eth_contract(
            bob_deploy_action.data,
            "0x0",
            bob_deploy_action.gas_limit
        );
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
        bob_fund_action.should.include.all.keys(
            "to",
            "data",
            "gas_limit",
            "value"
        );
        let { to, data, gas_limit, value } = bob_fund_action;
        let receipt = await bob.wallet.send_eth_transaction_to(
            to,
            data,
            value,
            gas_limit
        );
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
        alice_redeem_action.should.include.all.keys(
            "to",
            "data",
            "gas_limit",
            "value"
        );
        alice_erc20_balance_before = await test_lib.erc20_balance(
            alice_final_address,
            token_contract_address
        );
        await alice.wallet.send_eth_transaction_to(
            alice_redeem_action.to,
            alice_redeem_action.data,
            alice_redeem_action.value,
            alice_redeem_action.gas_limit
        );
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
        let alice_erc20_balance_after = await test_lib.erc20_balance(
            alice_final_address,
            token_contract_address
        );

        let alice_erc20_balance_expected = alice_erc20_balance_before.add(
            beta_asset_amount
        );
        alice_erc20_balance_after
            .toString()
            .should.be.equal(alice_erc20_balance_expected.toString());
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
        bob_redeem_action.should.include.all.keys("hex");
        bob_btc_balance_before = await test_lib.btc_balance(bob_final_address);
        await bob.wallet.send_raw_tx(bob_redeem_action.hex);
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
        let bob_btc_balance_after = await test_lib.btc_balance(
            bob_final_address
        );
        const bob_btc_balance_expected =
            bob_btc_balance_before + alpha_asset_amount - alpha_max_fee;
        bob_btc_balance_after.should.be.at.least(bob_btc_balance_expected);
    });
});
