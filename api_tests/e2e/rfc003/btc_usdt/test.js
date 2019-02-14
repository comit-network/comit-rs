const chai = require("chai");
chai.use(require("chai-http"));
const omnilayer = require("../../../lib/omnilayer.js");
const actor = require("../../../lib/actor.js");
const should = chai.should();
const logger = global.harness.logger;
const bitcoin = require("bitcoinjs-lib");

const omni_rpc_client = omnilayer.create_client();

const alice = actor.create("alice", {});
const bob = actor.create("bob", {});

const bob_final_address = "mzNFGtxdTSTJ1Lh6fq5N5oUgbhwA7Nm7cA";

const alpha_asset = 100000000;
const beta_asset = 5000;
const alpha_max_fee = 5000; // Max 5000 satoshis fee

const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

describe("RFC003: Bitcoin for USD Tether (Omnilayer)", () => {
    before(async function() {
        this.timeout(50000);
        //await omnilayer.activate_segwit();
        await alice.wallet.omni().btcFund(1);
        await bob.wallet.omni().btcFund(1);
        await omnilayer.omni_generate();
    });

    let tokenId;
    it("Create RegtestOmniCoin", async function() {
        const res = await alice.wallet.omni().createOmniToken();
        tokenId = res.propertyid;
    });

    let aliceOmniUTXO;
    it("Grant RegtestOmniCoin", async function() {
        aliceOmniUTXO = await alice.wallet.omni().grantOmniToken(tokenId, alice.wallet.omni().identity().output);
    });

    it("Swaperoo it", async function() {
        const aliceDetails = {
            alice_keypair: alice.wallet.omni().keypair,
            alice_omni_utxo: aliceOmniUTXO,
            alice_final_address: alice.wallet.omni().identity().address
        };
        const bobDetails = {
            bob_keypair: bob.wallet.omni().keypair,
            bob_btc_utxo: bob.wallet.omni().bitcoin_utxos.shift(),
            bob_btc_output: bob.wallet.omni().identity().output,
            bob_final_address: bob_final_address
        };

        const res = await omnilayer.swaperoo(aliceDetails, bobDetails, tokenId, 4200, 2);
    });
});
