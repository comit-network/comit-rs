const chai = require("chai");
chai.use(require("chai-http"));
const test_lib = require("../../../test_lib.js");
const should = chai.should();
const ethutil = require("ethereumjs-util");

const web3 = test_lib.web3();
const logger = test_lib.logger();

const toby_wallet = test_lib.wallet_conf();

const toby_initial_eth = "10";
const bob_initial_eth = "0.1";
const alice_initial_eth = "5";
const alice_initial_erc20 = web3.utils.toWei("10000", 'ether');

const alice = test_lib.comit_conf("alice", {}, 10009);
const bob = test_lib.comit_conf("bob", {}, 10019);

const alpha_asset_amount = new ethutil.BN(web3.utils.toWei("5000", 'ether'), 10);
const beta_asset = 4000000;

describe("RFC003: ERC20 for Lightning Bitcoin", () => {

    let token_contract_address;
    let alice_ln_info;
    let bob_ln_info;
    before(async function() {
        this.timeout(5000);

        alice_ln_info = await alice.ln.getInfo();
        bob_ln_info = await bob.ln.getInfo();
        const alice_ln_pubkey = alice_ln_info.identity_pubkey;
        const bob_ln_pubkey = bob_ln_info.identity_pubkey;
        await alice.ln.connectToPeer(bob_ln_pubkey, bob.ln.host);
        await bob.wallet.fund_btc(10);
        await bob.ln.send_btc_to_wallet(1);
        await test_lib.btc_generate(1);
        await bob.ln.openChannel(15000000, alice_ln_pubkey);
        let bob_channel_balance = await bob.ln.channelBalance();
        if (parseInt(bob_channel_balance.balance) === 0) {
            throw new Error("Bob should have some balance in a channel with Alice.");
        }
        await toby_wallet.fund_eth(toby_initial_eth);
        await alice.wallet.fund_eth(alice_initial_eth);
        await bob.wallet.fund_eth(bob_initial_eth);
        let receipt = await toby_wallet.deploy_erc20_token_contract();
        token_contract_address = receipt.contractAddress;
        await test_lib.btc_generate();
    });
    it(alice_initial_erc20 + " tokens were minted to Alice", async function() {
        const alice_eth_address = alice.wallet.eth_address();
        return test_lib
            .mint_erc20_tokens(
                toby_wallet,
                token_contract_address,
                alice_eth_address,
                alice_initial_erc20
            )
            .then(receipt => {
                receipt.status.should.equal(true);
                return test_lib.erc20_balance(alice_eth_address, token_contract_address)
                    .then(result => {
                        result = web3.utils.toBN(result).toString();
                        result.should.equal(alice_initial_erc20.toString());
                    });
            });
    });

    it("[Alice] Should be able to make a swap request via HTTP api", async () => {
        return chai.request(alice.comit_node_url())
            .post('/swaps/rfc003')
            .send({
                "alpha_ledger": {
                    "name": "Ethereum"
                },
                "beta_ledger": {
                    "name": "Lightning"
                },
                "alpha_asset": {
                    "name": "ERC20",
                    "quantity": alpha_asset_amount.toString(),
                    "token_contract" : token_contract_address
                },
                "beta_asset": {
                    "name": "Bitcoin",
                    "quantity": beta_asset.toString()
                },
                "alpha_ledger_refund_identity": alice.wallet.eth_address(),
                "beta_ledger_success_identity": alice_ln_info.identity_pubkey,
                "alpha_ledger_lock_duration": 86400
            }).then((res) => {
                res.should.have.status(201);
                swap_location = res.headers.location;
                logger.info("Alice created a new swap at %s", swap_location);
                swap_location.should.be.a("string");
                alice_swap_href = swap_location;
            });
        });

    let bob_swap_href;

    it("[Bob] Shows the Swap as Start in /swaps", async () => {
        // Bob has to wait for Lnd connection so he needs some wiggle room
        await test_lib.sleep(1000);
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
            alpha_ledger_success_identity: bob.wallet.eth_address(),
            beta_ledger_refund_identity: bob_ln_info.identity_pubkey,
            beta_ledger_lock_duration: 144,
        };

        logger.info("Bob is accepting the swap via %s with the following parameters", bob_accept_href, bob_response);

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

    let alice_add_invoice_href;
    it("[Alice] Can get the HTLC fund action", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_swap_href);
        res.should.have.status(200);
        res.body.state.should.equal("Start");
        let links = res.body._links;
        links.should.have.property("add_invoice");
        alice_add_invoice_href = links.add_invoice.href;
    });

    // let alice_funding_action;

    let alice_add_invoice_action;
    it("[Alice] Can get the invoice from the ‘add_invoice’ link", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_add_invoice_href);
        res.should.have.status(200);
        alice_add_invoice_action = res.body;

        logger.info("Alice retrieved the following add_invoice parameters", alice_add_invoice_action);
    });

    it("[Alice] Can execute the add_invoice action", async () => {
        alice_add_invoice_action.should.include.all.keys("r_preimage", "r_hash", "value");

        await alice.ln.addInvoice(
            alice_add_invoice_action.r_preimage,
            alice_add_invoice_action.r_hash,
            alice_add_invoice_action.value
        );
    });

    let alice_deploy_href;

    it("[Alice] Should be in Accepted state after executing the add_invoice action", async function() {
        this.timeout(10000);
        let swap = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "Accepted"
        );
        swap.should.have.property("_links");
        swap._links.should.have.property("deploy");
        alice_deploy_href = swap._links.deploy.href;
    });

    let alice_deploy_action;

    it("[Alice] Can get the deploy action from the ‘deploy’ link", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_deploy_href);
        res.should.have.status(200);
        alice_deploy_action = res.body;

        logger.info("Alice retrieved the following deployment parameters", alice_deploy_action);
    });

    it("[Alice] Can execute the deploy action", async () => {
        alice_deploy_action.should.include.all.keys("data", "gas_limit", "value");
        alice_deploy_action.value.should.equal("0");
        await alice.wallet.deploy_eth_contract(alice_deploy_action.data, "0x0", alice_deploy_action.gas_limit);
    });

    it("[Bob] Should be in AlphaDeployed state after Alice executes the deploy action", async function() {
        this.timeout(10000);
        let swap = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "AlphaDeployed"
        );
    });

    let alice_fund_href;

    it("[Alice] Should be in AlphaDeployed state after executing the deploy action", async function() {
        this.timeout(10000);
        let swap = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "AlphaDeployed"
        );
        let links = swap._links;
        links.should.have.property("fund");
        alice_fund_href = links.fund.href;
    });

    it("[Alice] Can get the fund action from the ‘fund’ link", async () => {
        let res = await chai
            .request(alice.comit_node_url())
            .get(alice_fund_href);
        res.should.have.status(200);
        alice_fund_action = res.body;

        logger.info("Alice retrieved the following funding parameters", alice_fund_action);
    });

    it("[Alice] Can execute the fund action", async () => {
        alice_fund_action.should.include.all.keys("to", "data", "gas_limit", "value");
        let { to, data, gas_limit, value } = alice_fund_action;
        let receipt = await alice.wallet.send_eth_transaction_to(to, data, value, gas_limit);
        receipt.status.should.equal(true);
    });

    
    it("[Alice] Should be in AlphaFunded state after executing the fund action", async function() {
        this.timeout(10000);
        let swap = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "AlphaFunded"
        );
    });

    let bob_fund_href;

    it("[Bob] Should be in AlphaFunded state after Alice executes the fund action", async function() {
        this.timeout(10000);
        let swap = await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "AlphaFunded"
        );
        swap._links.should.have.property("fund");
        bob_fund_href = swap._links.fund.href;
    });

    let bob_fund_action;
    
    it("[Bob] Can get the funding action from the ‘fund’ link", async () => {
        let res = await chai
            .request(bob.comit_node_url())
            .get(bob_fund_href);
        res.should.have.status(200);
        bob_fund_action = res.body;

        logger.info("Bob retrieved the following funding parameters", bob_fund_action);
    });

    let alice_channel_balance_before;

    it("[Bob] Can execute the funding action", async () => {
        bob_fund_action.should.include.all.keys("dest", "amt", "payment_hash", "final_cltv_delta");
        alice_channel_balance_before = await alice.ln.channelBalance();
        alice_channel_balance_before = parseInt(alice_channel_balance_before.balance);
        let result = await bob.ln.sendPaymentSync(
            bob_fund_action.dest,
            bob_fund_action.amt,
            bob_fund_action.payment_hash,
            bob_fund_action.final_cltv_delta,
        );
    });

    it("[Alice] Should be in AlphaFundedBetaRedeemed state after Bob executes the fund action", async function() {
        this.timeout(10000);
        let swap = await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "AlphaFundedBetaRedeemed"
        );
    });

    it("[Alice] Should have received the beta asset after the redeem", async function() {
        let alice_channel_balance_after = await alice.ln.channelBalance();
        alice_channel_balance_after = parseInt(alice_channel_balance_after.balance);
        
        let alice_channel_balance_expected = alice_channel_balance_before + beta_asset;
        alice_channel_balance_after.should.be.equal(alice_channel_balance_expected);
    });

    let bob_redeem_href;

    it("[Bob] Should be in AlphaFundedBetaRedeemed state after executing the fund action", async function() {
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
            .get(bob_redeem_href);
        res.should.have.status(200);
        bob_redeem_action = res.body;

        logger.info("Bob retrieved the following redeem parameters", bob_redeem_action);
    });

    let bob_erc20_balance_before;

    it("[Bob] Can execute the redeem action", async function () {
        bob_redeem_action.should.include.all.keys("to", "data", "gas_limit", "value");
        bob_erc20_balance_before = await test_lib.erc20_balance(bob.wallet.eth_address(), token_contract_address);
        await bob.wallet.send_eth_transaction_to(
            bob_redeem_action.to,
            bob_redeem_action.data,
            bob_redeem_action.value,
            bob_redeem_action.gas_limit);
    });

    it("[Bob] Should be in BothRedeemed state after executing the redeem action", async function() {
        this.timeout(10000);
        await bob.poll_comit_node_until(
            chai,
            bob_swap_href,
            "BothRedeemed"
        );
    });

    it("[Bob] Should have received the alpha asset after the redeem", async function() {
        let bob_erc20_balance_after = await test_lib.erc20_balance(bob.wallet.eth_address(), token_contract_address);
        let bob_erc20_balance_expected = bob_erc20_balance_before.add(alpha_asset_amount);
        bob_erc20_balance_after.toString().should.be.equal(bob_erc20_balance_expected.toString());
    });

    it("[Alice] Should be in BothRedeemed state after Bob executes the redeem action", async function() {
        this.timeout(10000);
        await alice.poll_comit_node_until(
            chai,
            alice_swap_href,
            "BothRedeemed"
        );
    });


});
