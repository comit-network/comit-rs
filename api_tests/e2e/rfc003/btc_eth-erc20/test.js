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

const toby = test_lib.wallet_conf();

//Alice
const alice_initial_eth = "0.1";
const alice = test_lib.comit_conf("alice");
const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

//Bob
const bob = test_lib.comit_conf("bob");
const bob_initial_eth = 5;
const bob_initial_erc20 = 10000;
const bob_config = Toml.parse(fs.readFileSync(process.env.BOB_CONFIG_FILE, 'utf8'));
const bob_eth_private_key = Buffer.from(bob_config.ethereum.private_key, "hex");
const bob_eth_address = "0x" + ethutil.privateToAddress(bob_eth_private_key).toString("hex");

const beta_asset_amount = web3.utils.toWei("5000", 'ether');

describe('RFC003: Bitcoin for ERC20', () => {
    let token_contract_address;
    before(async function() {
        this.timeout(5000);
        await test_lib.fund_eth(20);
        await test_lib.give_eth_to(toby.eth_address(), 10);
        await test_lib.give_eth_to(bob.eth_address(), bob_initial_eth);
        let receipt = await test_lib.deploy_erc20_token_contract(toby);
        token_contract_address = receipt.contractAddress;
    });

    it(bob_initial_erc20 + " tokens were minted to Bob", async function() {
        return test_lib
            .mint_erc20_tokens(
                toby,
                token_contract_address,
                bob.eth_address(),
                bob_initial_erc20
            )
            .then(receipt => {
                receipt.status.should.equal(true);
                return bob
                    .erc20_balance(token_contract_address)
                    .then(result => {
                        result = web3.utils.toBN(result).toString();
                        result.should.equal(bob_initial_erc20.toString());
                    });
            });
    });

    let alice_swap_id;
    let swap_location;
    it("Alice should be able to make a swap request via HTTP api", async () => {
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
                "alpha_ledger_refund_identity": "ac2db2f2615c81b83fe9366450799b4992931575",
                "beta_ledger_success_identity": alice_final_address,
                "alpha_ledger_lock_duration": 144
            }).then((res) => {
                res.should.have.status(201);
                swap_location = res.headers.location;
                swap_location.should.be.a('string');
                alice_swap_id = res.body.id;
            });
    });

    it("[Alice] Shows the Swap as Start at /swaps/rfc003/:id.", async () => {
        let res = await chai.request(alice.comit_node_url())
            .get(swap_location);
        res.body.role.should.equal('Alice');
        console.log(res.body);
    });

});
