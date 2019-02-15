const chai = require("chai");
chai.use(require("chai-http"));
const omnilayer = require("../../../lib/omnilayer.js");
const actor = require("../../../lib/actor.js");
const should = chai.should();

const alice = actor.create("alice", {});
const bob = actor.create("bob", {});

const bob_final_address = "mzNFGtxdTSTJ1Lh6fq5N5oUgbhwA7Nm7cA";

const beta_asset = 3;
const alpha_asset = 4200;

describe("RFC003: Bitcoin for Reg Test Omni Token (USD Tether style)", () => {
    before(async function() {
        this.timeout(50000);
        await omnilayer.activateSegwit();
        await alice.wallet.omni().btcFund(1);
        await bob.wallet.omni().btcFund(1);
        await omnilayer.generate();
    });

    let tokenId;
    it("Create Regtest Omni Token", async function() {
        const res = await alice.wallet.omni().createOmniToken();
        res.propertyid.should.be.a("number");
        tokenId = res.propertyid;
    });

    let aliceOmniUTXO;
    it("Grant Regtest Omni Token", async function() {
        const grantAmount = alpha_asset * 3;
        aliceOmniUTXO = await alice.wallet.omni().grantOmniToken(tokenId, alice.wallet.omni().identity().output, grantAmount);
        const balance = await omnilayer.getBalance(tokenId, alice.wallet.omni().identity().address);
        balance.should.equal(grantAmount.toString());
        const bob_omni_balance = await omnilayer.getBalance(tokenId, bob_final_address);
        bob_omni_balance.should.equal("0");
    });

    it("Swaperoo it", async function() {
        const aliceDetails = {
            alice_keypair: alice.wallet.omni().keypair,
            alice_omni_utxo: aliceOmniUTXO,
            alice_final_address: alice.wallet.omni().identity().address,
        };
        const bobDetails = {
            bob_keypair: bob.wallet.omni().keypair,
            bob_btc_utxo: bob.wallet.omni().bitcoin_utxos.shift(),
            bob_btc_output: bob.wallet.omni().identity().output,
            bob_final_address: bob_final_address,
        };

        await omnilayer.swaperoo(aliceDetails, bobDetails, tokenId, alpha_asset, beta_asset);

        const bob_omni_balance = await omnilayer.getBalance(tokenId, bob_final_address);
        bob_omni_balance.should.equal(alpha_asset.toString());
    });
});
