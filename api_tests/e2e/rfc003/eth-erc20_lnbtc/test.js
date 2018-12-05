const chai = require("chai");
chai.use(require("chai-http"));
const test_lib = require("../../../test_lib.js");
const should = chai.should();
const ethutil = require("ethereumjs-util");

const web3 = test_lib.web3();
const logger = test_lib.logger();

const bob_initial_eth = "0.1";
const alice_initial_eth = "0.2";
const alice_initial_erc20 = "420000";

const alice = test_lib.comit_conf("alice", {}, 10009);
const bob = test_lib.comit_conf("bob", {}, 10019);

const bob_final_address = "0x03a329c0248369a73afac7f9381e02fb43d2ea72";

const alpha_asset = new ethutil.BN(web3.utils.toWei("4.5", "ether"), 10);
const beta_asset = 230000000;

const toby_wallet = test_lib.wallet_conf();
const alpha_asset_amount = new ethutil.BN(web3.utils.toWei("5000", 'ether'), 10);



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
        await bob.wallet.fund_btc(5);
        await bob.ln.send_btc_to_wallet(3);
        await test_lib.btc_generate(1);
        await bob.ln.openChannel(7000000, alice_ln_pubkey);
        let bob_channel_balance = await bob.ln.channelBalance();
        if (parseInt(bob_channel_balance.balance) === 0) {
            throw new Error("Bob should have some balance in a channel with Alice.");
        }
        await toby_wallet.fund_eth(10);
        await alice.wallet.fund_eth(alice_initial_eth);
        await bob.wallet.fund_eth(bob_initial_eth);
        let receipt = await toby_wallet.deploy_erc20_token_contract();
        token_contract_address = receipt.contractAddress;
        await test_lib.btc_generate();
    });
    it(alice_initial_erc20 + " tokens were minted to Alice", async function() {
        const alice_eth_address = alice.wallet.eth_address()
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
                    "name": "Ethereum",
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
                "alpha_ledger_lock_duration": 86400,
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

    // it("[Alice] Can execute the funding action", async () => {
    //     alice_funding_action.should.include.all.keys("address", "value");

    //     await alice.wallet.send_btc_to_address(
    //         alice_funding_action.address,
    //         parseInt(alice_funding_action.value)
    //     );
    // });

    // it("[Alice] Should be in AlphaFunded state after executing the funding action", async function() {
    //     this.timeout(10000);
    //     await alice.poll_comit_node_until(
    //         chai,
    //         alice_swap_href,
    //         "AlphaFunded"
    //     );
    // });

});
