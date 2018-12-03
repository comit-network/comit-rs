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

describe("RFC003: ERC20 for Lightning Bitcoin", () => {

    let token_contract_address;
    before(async function() {
        this.timeout(5000);

        const alice_info = await alice.ln.getInfoAsync();
        const bob_info = await bob.ln.getInfoAsync();
        const alice_ln_pubkey = alice_info.identity_pubkey;
        const bob_ln_pubkey = bob_info.identity_pubkey;
        await alice.ln.connectToPeerAsync(bob_ln_pubkey, bob.ln.host);
        await bob.wallet.fund_btc(5);
        await bob.ln.send_btc_to_wallet(3);
        await test_lib.btc_generate(1);
        await bob.ln.openChannelAsync(7000000, alice_ln_pubkey);
        let bob_channel_balance = await bob.ln.channelBalanceAsync();
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

});
