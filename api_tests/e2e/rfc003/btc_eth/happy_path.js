const chai = require('chai');
const BigNumber = require('bignumber.js');
chai.use(require('chai-http'));
const Toml = require('toml');
const comit_test = require("../../../comit_test.js");
const should = chai.should();
const EthereumTx = require('ethereumjs-tx');
const assert = require('assert');
const fs = require('fs');
const ethutil = require('ethereumjs-util');


const web3 = comit_test.web3();

const bob_initial_eth = "11";
const bob_config = Toml.parse(fs.readFileSync(process.env.BOB_CONFIG_FILE, 'utf8'));
const bob_eth_private_key = Buffer.from(bob_config.ethereum.private_key,"hex");
const bob_eth_address = "0x" + ethutil.privateToAddress(bob_eth_private_key).toString("hex");
const alice_initial_eth = "0.1";
const alice = comit_test.player_conf("alice", {
    txid: process.env.BTC_FUNDED_TX,
    value: parseInt(process.env.BTC_FUNDED_AMOUNT + '00000000'),
    private_key: process.env.BTC_FUNDED_PRIVATE_KEY,
    vout: parseInt(process.env.BTC_FUNDED_VOUT)
});

const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
const target_asset = new BigNumber(web3.utils.toWei("10", 'ether'));
const bitcoin_rpc_client = comit_test.bitcoin_rpc_client();

describe('RFC003 Bitcoin for Ether', () => {

    before(() => {
        Promise.all(
            [
                comit_test.give_eth_to(bob_eth_address, bob_initial_eth),
                comit_test.give_eth_to(alice.eth_address(), alice_initial_eth)
            ]
        ).then(receipt => {
            console.log(`Giving ${bob_initial_eth} Ether to Bob; success : ${receipt[0].status}`);
            console.log(`Giving ${alice_initial_eth} Ether to Alice; success : ${receipt[1].status}`);
        }).catch(error => {
            console.log(`Error on giving Ether to Alice or Bob : ${error}`);
        });
    });

    let alice_swap_id;
    it("Alice should be able to make a swap request via HTTP api", async () => {
        return chai.request(alice.comit_node_url())
            .post('/swaps')
            .send({
                "source_ledger"  : {
                    "value" : "Bitcoin",
                    "identity" : "ac2db2f2615c81b83fe9366450799b4992931575",
                },
                "target_ledger" : {
                    "value" : "Ethereum",
                    "identity" : alice_final_address,
                },
                "source_asset" : {
                    "value" : "Bitcoin",
                    "quantity" : "100000000"
                },
                "target_asset" : {
                    "value" : "Ether",
                    "quantity" : target_asset.toString(),
                }
            }).then((res) => {
                res.should.have.status(201);
                alice_swap_id = res.body.id;
            });
    });

    let alice_funding_required;

    it("The request should eventually be accepted by Bob", function (done) {
        this.timeout(10000);
        alice.poll_until(chai, alice_swap_id, "accepted").then((status) => {
            alice_funding_required = status.funding_required;
            done();
        });
    });

    it("Alice should be able to manually fund the bitcoin HTLC", async () => {
        return alice.send_btc_to_address(alice_funding_required, 100000000);
    });

    let redeem_details;

    it("Bob should eventually deploy the Ethereum HTLC and Alice should see it", function (done) {
        this.timeout(10000);
        alice.poll_until(chai, alice_swap_id, "redeemable").then((status) => {
            redeem_details = status;
            done();
        });
    });

    it("Alice should be able to redeem Ether", async function() {
        this.timeout(10000);
        await comit_test.sleep(2000);
        let old_balance = new BigNumber(await web3.eth.getBalance(alice_final_address));
        await alice.send_eth_transaction_to(redeem_details.contract_address,  "0x" + redeem_details.data);
        await comit_test.sleep(2000);
        let new_balance = new BigNumber(await web3.eth.getBalance(alice_final_address));
        let diff = new_balance.minus(old_balance);
        diff.toString().should.equal(target_asset.toString());
    });
});
