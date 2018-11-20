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

const bob_initial_eth = "11";
const bob_config = Toml.parse(fs.readFileSync(process.env.BOB_CONFIG_FILE, 'utf8'));
const bob_eth_private_key = Buffer.from(bob_config.ethereum.private_key, "hex");
const bob_eth_address = "0x" + ethutil.privateToAddress(bob_eth_private_key).toString("hex");
const alice_initial_eth = "0.1";
const alice = test_lib.comit_conf("alice", {
    txid: process.env.BTC_FUNDED_TX,
    value: parseInt(process.env.BTC_FUNDED_AMOUNT + '00000000'),
    private_key: process.env.BTC_FUNDED_PRIVATE_KEY,
    vout: parseInt(process.env.BTC_FUNDED_VOUT)
});

const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const target_asset = new BigNumber(web3.utils.toWei("10", 'ether'));
const bitcoin_rpc_client = test_lib.bitcoin_rpc_client();

describe('RFC003 Bitcoin for Ether', () => {

    before(async function () {
        this.timeout(3000);
        return await test_lib.fund_eth(20).then(async () => {
            return await Promise.all(
                [
                    test_lib.give_eth_to(bob_eth_address, bob_initial_eth),
                    test_lib.give_eth_to(alice.wallet.eth_address(), alice_initial_eth)
                ]
            );
        });
    });

    let alice_swap_id;
    let swap_location;
    it("Alice should be able to make a swap request via HTTP api", async () => {
        return chai.request(alice.comit_node_url())
            .post('/swaps/rfc003')
            .send({
                "source_ledger": {
                    "name": "Bitcoin",
                    "network": "regtest"
                },
                "target_ledger": {
                    "name": "Ethereum"
                },
                "source_asset": {
                    "name": "Bitcoin",
                    "quantity": "100000000"
                },
                "target_asset": {
                    "name": "Ether",
                    "quantity": target_asset.toString(),
                },
                "source_ledger_refund_identity": "ac2db2f2615c81b83fe9366450799b4992931575",
                "target_ledger_success_identity": alice_final_address,
                "source_ledger_lock_duration": 144
            }).then((res) => {
                res.should.have.status(201);
                swap_location = res.headers.location;
                swap_location.should.be.a('string');
                alice_swap_id = res.body.id;
            });
    });

    let alice_funding_required;

    it("The request should eventually be accepted by Bob", function (done) {
        this.timeout(10000);
        alice.poll_comit_node_until(chai, swap_location, "accepted").then((status) => {
            alice_funding_required = status.funding_required;
            done();
        });
    });

    it("Alice should be able to manually fund the bitcoin HTLC", async function () {
        this.slow(500);
        return alice.wallet.send_btc_to_p2wsh_address(alice_funding_required, 100000000);
    });

    let redeem_details;

    it("Bob should eventually deploy the Ethereum HTLC and Alice should see it", function (done) {
        this.slow(7000);
        this.timeout(10000);
        alice.poll_comit_node_until(chai, swap_location, "redeemable").then((status) => {
            redeem_details = status;
            done();
        });
    });

    it("Alice should be able to redeem Ether", async function () {
        this.slow(6000);
        this.timeout(10000);
        await test_lib.sleep(2000);
        let old_balance = new BigNumber(await web3.eth.getBalance(alice_final_address));
        await alice.wallet.send_eth_transaction_to(redeem_details.contract_address, "0x" + redeem_details.data);
        await test_lib.sleep(2000);
        let new_balance = new BigNumber(await web3.eth.getBalance(alice_final_address));
        let diff = new_balance.minus(old_balance);
        diff.toString().should.equal(target_asset.toString());
    });
});
