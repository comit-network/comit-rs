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

const toby_eth_private_key = Buffer.from(
  "fed52717ddb17a45e718a0903024224ab69a2456157aaa16e606e65b9943c899",
  "hex"
);

const toby = test_lib.wallet_conf(toby_eth_private_key, {
  txid: process.env.BTC_FUNDED_TX,
  value: parseInt(process.env.BTC_FUNDED_AMOUNT + "00000000"),
  private_key: process.env.BTC_FUNDED_PRIVATE_KEY,
  vout: parseInt(process.env.BTC_FUNDED_VOUT)
});

const alice_initial_eth = "0.1";
const alice = test_lib.comit_conf("alice", {
  txid: process.env.BTC_FUNDED_TX,
  value: parseInt(process.env.BTC_FUNDED_AMOUNT + "00000000"),
  private_key: process.env.BTC_FUNDED_PRIVATE_KEY,
  vout: parseInt(process.env.BTC_FUNDED_VOUT)
});
const alice_final_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

const bob_initial_eth = 5;
const bob_initial_erc20 = 10000;
const bob_config = Toml.parse(
  fs.readFileSync(process.env.BOB_CONFIG_FILE, "utf8")
);
const bob_eth_private_key = Buffer.from(bob_config.ethereum.private_key, "hex");
const bob_eth_address =
  "0x" + ethutil.privateToAddress(bob_eth_private_key).toString("hex");
const bob = test_lib.wallet_conf(bob_eth_private_key, null);

const beta_asset_amount = new BigNumber(web3.utils.toWei("5000", "ether"));

describe("RFC003: Bitcoin for ERC20", () => {
  let token_contract_address;
  before(async function() {
    this.timeout(5000);
    await test_lib.fund_eth(20);
    await test_lib.give_eth_to(toby.eth_address(), 10);
    await test_lib.give_eth_to(bob_eth_address, bob_initial_eth);
    await test_lib.deploy_erc20_token_contract(toby).then(receipt => {
      token_contract_address = receipt.contractAddress;
    });
    await test_lib.give_eth_to(alice.wallet.eth_address(), alice_initial_eth);
  });

  it("The token contract address is as predicted", async function() {
    return token_contract_address.should.equal(
      "0x0c4526600167e15124350e6921A889D7D5778Aa2"
    );
  });

  it(bob_initial_erc20 + " tokens were minted to Bob", async function() {
    return test_lib
      .mint_erc20_tokens(
        toby,
        token_contract_address,
        bob_eth_address,
        bob_initial_erc20
      )
      .then(receipt => {
        receipt.status.should.equal(true);
        return bob.erc20_balance(token_contract_address).then(result => {
          result = web3.utils.toBN(result).toString();
          result.should.equal(bob_initial_erc20.toString());
        });
      });
  });

  let alice_swap_id;
  let swap_location;
  it("Alice should be able to make a swap request via HTTP api", async () => {
    return chai
      .request(alice.comit_node_url())
      .post("/swaps/rfc003")
      .send({
        alpha_ledger: {
          name: "Bitcoin",
          network: "regtest"
        },
        beta_ledger: {
          name: "Ethereum"
        },
        alpha_asset: {
          name: "Bitcoin",
          quantity: "100000000"
        },
        beta_asset: {
          name: "PAY",
          quantity: beta_asset.toString()
        },
        alpha_ledger_refund_identity:
          "ac2db2f2615c81b83fe9366450799b4992931575",
        beta_ledger_success_identity: alice_final_address,
        alpha_ledger_lock_duration: 144
      })
      .then(res => {
        res.should.have.status(201);
        swap_location = res.headers.location;
        swap_location.should.be.a("string");
        alice_swap_id = res.body.id;
      });
  });
});
