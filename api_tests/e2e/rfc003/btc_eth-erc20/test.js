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

const toby = test_lib.wallet_conf(toby_eth_private_key, {
    txid: process.env.BTC_FUNDED_TX,
    value: parseInt(process.env.BTC_FUNDED_AMOUNT + '00000000'),
    private_key: process.env.BTC_FUNDED_PRIVATE_KEY,
    vout: parseInt(process.env.BTC_FUNDED_VOUT)
})

describe('RFC003: BTC -> Bitcoin for ERC20', () => {

    var token_contract_address;
    before(async function () {
        return test_lib.fund_eth(20).then(() => {
            console.log(`Gave 20 Ether to funded address`);
            return test_lib.give_eth_to(toby.eth_address(), 10)
                .then(receipt => {
                    console.log(`Giving 20 Ether to Toby; success: ${receipt.status}`);
                    return test_lib.deploy_erc20_token_contract(toby).then(receipt => {
                        token_contract_address = receipt.contractAddress;
                        console.log(`Deploying ERC20 token contract; success: ${receipt.status}`);
                    });
                }).catch(error => {
                    console.log(`Error on giving Ether to Toby: ${error}`);
                });
        }).catch(error => {
            console.log(`Error on funding Ether: ${error}`);
        });
    });

    it("The token contract address is as predicted", async function () {
        return token_contract_address.should.equal("0x0c4526600167e15124350e6921A889D7D5778Aa2");
    });

});
