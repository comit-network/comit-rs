const chai = require("chai");
const BigNumber = require("bignumber.js");
chai.use(require("chai-http"));
const Toml = require("toml");
const test_lib = require("../../../test_lib.js");
const should = chai.should();
const EthereumTx = require("ethereumjs-tx");
const assert = require("assert");
const fs = require("fs");
const ethutil = require("ethereumjs-util");

const web3 = test_lib.web3();

const toby = test_lib.wallet_conf();
const bob_initial_eth = 5;
const bob_initial_erc20 = 10000;
const bob_config = Toml.parse(
    fs.readFileSync(process.env.BOB_CONFIG_FILE, "utf8")
);
const bob = test_lib.wallet_conf();

describe("RFC003: Bitcoin for ERC20", () => {
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
});
