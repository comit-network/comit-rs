const chai = require('chai');
const BigNumber = require('bignumber.js');
chai.use(require('chai-http'));
const Toml = require('toml');
const test_lib = require("../../../test_lib.js");
const should = chai.should();
const EthereumTx = require('ethereumjs-tx');
const assert = require('assert');
const fs = require('fs');
const ethutil = require('ethereumjs-util');

const web3 = test_lib.web3();

const toby_wallet = test_lib.wallet_conf();

//Alice
const alice_initial_eth = "0.1";
const alice = test_lib.comit_conf("alice");
const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

//Bob
const bob = test_lib.comit_conf("bob");
const bob_initial_eth = 5;
const bob_initial_erc20 = web3.utils.toWei("10000", 'ether');
const bob_config = Toml.parse(fs.readFileSync(process.env.BOB_CONFIG_FILE, 'utf8'));

const beta_asset_amount = web3.utils.toWei("5000", 'ether');

describe('RFC003: Bitcoin for ERC20', () => {
    let token_contract_address;
    before(async function() {
        this.timeout(5000);
        await toby_wallet.fund_eth(10);
        await bob.wallet.fund_eth(bob_initial_eth);
        await alice.wallet.fund_btc(10);
        await alice.wallet.fund_eth(1);
        let receipt = await toby_wallet.deploy_erc20_token_contract();
        token_contract_address = receipt.contractAddress;
    });

    it(bob_initial_erc20 + " tokens were minted to Bob", async function() {
        let receipt = await test_lib
            .mint_erc20_tokens(
                toby_wallet,
                token_contract_address,
                bob.wallet.eth_address(),
                bob_initial_erc20
            );

        receipt.status.should.equal(true);

        let erc20_balance = await bob.wallet.erc20_balance(token_contract_address);
        erc20_balance.toString().should.equal(bob_initial_erc20);
    });

    let swap_location;
    let alice_swap_href;
    it("[Alice] Should be able to make a swap request via HTTP api", async () => {
        return chai.request(alice.comit_node_url())
            .post('/swaps/rfc003')
            .send({
                "alpha_ledger": {
                    "name": "Bitcoin",
                    "network": "regtest"
                },
                "beta_ledger": {
                    "name": "Ethereum"
                },
                "alpha_asset": {
                    "name": "Bitcoin",
                    "quantity": "100000000"
                },
                "beta_asset": {
                    "name": "ERC20",
                    "quantity": beta_asset_amount,
                    "token_contract" : token_contract_address,
                },
                "alpha_ledger_refund_identity": null,
                "beta_ledger_success_identity": alice_final_address,
                "alpha_ledger_lock_duration": 144
            }).then((res) => {
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
        let res = await chai
            .request(bob.comit_node_url())
            .get(bob_deploy_href);
        res.should.have.status(200);
        bob_deploy_action = res.body;
    });


    it("[Bob] Can execute the deploy action", async () => {
        bob_deploy_action.should.include.all.keys("data", "gas_limit", "value");
        bob_deploy_action.value.should.equal("0");
        await bob.wallet.deploy_eth_contract(bob_deploy_action.data, "0x0", bob_deploy_action.gas_limit);
    });

    it("[Alice] Should be in AlphaFundedBetaDeployed state after Bob executes the funding action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "AlphaFundedBetaDeployed"
        );
    });


    let bob_fund_href;
    it("[Bob] Should be in AlphaFundedBetaDeployed state after executing the funding action", async function() {
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
        let res = await chai
            .request(bob.comit_node_url())
            .get(bob_fund_href);
        res.should.have.status(200);
        bob_fund_action = res.body;
    });

    it("[Bob] Can execute the fund action", async () => {
        bob_fund_action.should.include.all.keys("to", "data", "gas_limit", "value");
        let { to, data, gas_limit, value } = bob_fund_action;
        let receipt = await bob.wallet.send_eth_transaction_to(to, data, value, gas_limit);
        receipt.status.should.equal(true);
        let erc20_balance = await bob.wallet.erc20_balance(token_contract_address);
    });

    it("[Bob] Should be in BothFunded state after executing the funding action", async function() {
        this.timeout(100000000);
        await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "BothFunded"
        );
    });

    let alice_redeem_href;
    it("[Alice] Should be in BothFunded state after Bob executes the funding action", async function() {
        this.timeout(100000000);
        let swap = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "BothFunded"
        );
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
    });

    it("[Alice] Can execute the redeem action", async function () {
        alice_redeem_action.should.include.all.keys("to", "data", "gas_limit", "value");
        await alice.wallet.send_eth_transaction_to(
                alice_redeem_action.to,
                alice_redeem_action.data,
                alice_redeem_action.value,
                alice_redeem_action.gas_limit);
    });

});
