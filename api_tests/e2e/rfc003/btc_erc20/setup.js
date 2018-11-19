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

const toby_eth_private_key = Buffer.from("fed52717ddb17a45e718a0903024224ab69a2456157aaa16e606e65b9943c899", "hex");
const toby_eth_address = "0x" + ethutil.privateToAddress(toby_eth_private_key).toString("hex");

const toby = test_lib.wallet_conf(toby_eth_private_key, {
    txid: process.env.BTC_FUNDED_TX,
    value: parseInt(process.env.BTC_FUNDED_AMOUNT + '00000000'),
    private_key: process.env.BTC_FUNDED_PRIVATE_KEY,
    vout: parseInt(process.env.BTC_FUNDED_VOUT)
})

describe('RFC003: Setup ERC20 token contract', () => {

    before(() => {
        test_lib.fund_eth(20).then(() => {
            console.log(`Gave 20 Ether to funded address`);
            test_lib.give_eth_to(toby_eth_address, 10)
                .then(receipt => {
                    console.log(`Giving 10 Ether to Toby; success: ${receipt[0].status}`);
                }).catch(error => {
                console.log(`Error on giving Ether to Toby: ${error}`);
            });
        }).catch(error => {
            console.log(`Error on funding Ether: ${error}`);
        });
    });

    it("Creation of new token contract should be successful", async function () {
        return toby.send_eth_transaction_to("").then(receipt => {
            console.log(receipt);
        });
    });
});
