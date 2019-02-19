const chai = require("chai");
chai.use(require("chai-http"));
const omnilayer = require("../../../lib/omnilayer.js");
const actor = require("../../../lib/actor.js");
const should = chai.should();

describe("Bitcoin for Omni Token (USD Tether style)", () => {
    const alice = actor.create("alice", {});
    const bob = actor.create("bob", {});
    const bob_final_address = "mzNFGtxdTSTJ1Lh6fq5N5oUgbhwA7Nm7cA";
    const beta_asset = 3;
    const alpha_asset = 4200;

    before(async function() {
        this.timeout(50000);
        await omnilayer.activateSegwit();
        await alice.wallet.omni().btcFund(1, true);
        await bob.wallet.omni().btcFund(1, true);
        await omnilayer.generate();
    });

    let tokenId;
    it("Create Regtest Omni Token", async function() {
        const res = await omnilayer.createOmniToken(
            "Regtest Token",
            alice.wallet.omni().keypair,
            alice.wallet.omni().bitcoin_utxos.shift(),
            alice.wallet.omni().identity(true).output,
        );
        res.propertyid.should.be.a("number");
        tokenId = res.propertyid;
    });

    let aliceOmniUTXO;
    it("Grant Regtest Omni Token", async function() {
        const grantAmount = alpha_asset * 3;

        aliceOmniUTXO = await omnilayer.grantOmniToken(
            alice.wallet.omni().keypair,
            alice.wallet.omni().bitcoin_utxos.shift(),
            alice.wallet.omni().output,
            tokenId,
            alice.wallet.omni().identity(true).output,
            grantAmount);


        const balance = await omnilayer.getBalance(tokenId, alice.wallet.omni().identity(true).address);
        balance.should.equal(grantAmount.toString());
        const bob_omni_balance = await omnilayer.getBalance(tokenId, bob_final_address);
        bob_omni_balance.should.equal("0");
    });

    it("Swaperoo it", async function() {
        const aliceDetails = {
            alice_keypair: alice.wallet.omni().keypair,
            alice_omni_utxo: aliceOmniUTXO,
            alice_final_address: alice.wallet.omni().identity(true).address,
        };
        const bobDetails = {
            bob_keypair: bob.wallet.omni().keypair,
            bob_btc_utxo: bob.wallet.omni().bitcoin_utxos.shift(),
            bob_btc_output: bob.wallet.omni().identity(true).output,
            bob_final_address: bob_final_address,
        };

        await omnilayer.swaperoo(aliceDetails, bobDetails, tokenId, alpha_asset, beta_asset);

        const bob_omni_balance = await omnilayer.getBalance(tokenId, bob_final_address);
        bob_omni_balance.should.equal(alpha_asset.toString());
    });
});

describe("Omni Token (USD Tether style) transferred through P2SH HTLC", () => {
    const alice = actor.create("alice", {});
    const bob = actor.create("bob", {});
    const bob_final_address = "mh9g3jCJxkc4tzV88THmQHGNGiCzUZ1zg6";
    const beta_asset = 4;
    const alpha_asset = 5300;

    before(async function() {
        this.timeout(50000);
        await omnilayer.generate();
        await alice.wallet.omni().btcFund(1, true);
        await bob.wallet.omni().btcFund(1, true);
        await omnilayer.generate();
    });

    let tokenId;
    it("Create Regtest Omni Token", async function() {
        const res = await omnilayer.createOmniToken(
            "Regtest Token for HTLC",
            alice.wallet.omni().keypair,
            alice.wallet.omni().bitcoin_utxos.shift(),
            alice.wallet.omni().identity(true).output,
        );
        res.propertyid.should.be.a("number");
        tokenId = res.propertyid;
    });

    let aliceOmniUTXO;
    it("Grant Regtest Omni Token", async function() {
        const grantAmount = alpha_asset * 3;

        aliceOmniUTXO = await omnilayer.grantOmniToken(
            alice.wallet.omni().keypair,
            alice.wallet.omni().bitcoin_utxos.shift(),
            alice.wallet.omni().output,
            tokenId,
            alice.wallet.omni().identity(true).output,
            grantAmount);


        const balance = await omnilayer.getBalance(tokenId, alice.wallet.omni().identity(true).address);
        balance.should.equal(grantAmount.toString());
        const bob_omni_balance = await omnilayer.getBalance(tokenId, bob_final_address);
        bob_omni_balance.should.equal("0");
    });

    let htlcUTXO;
    it("Lock Regtest Omni Token in HTLC", async function() {
        const aliceDetails = {
            alice_keypair: alice.wallet.omni().keypair,
            alice_omni_utxo: aliceOmniUTXO,
            alice_final_address: alice.wallet.omni().identity(true).address,
        };
        const bobDetails = {
            bob_keypair: bob.wallet.omni().keypair,
            bob_btc_output: bob.wallet.omni().identity(true).output,
            bob_final_address: bob_final_address,
        };

        htlcUTXO = await omnilayer.lockInHTLC(aliceDetails, bobDetails, tokenId, alpha_asset);

        const htlcBalance = await omnilayer.getBalance(tokenId, htlcUTXO.address);
        htlcBalance.should.equal(alpha_asset.toString());
    });

    it("Redeem Regtest Omni Token from HTLC", async function() {
        const bobDetails = {
            bob_keypair: bob.wallet.omni().keypair,
            bob_btc_output: bob.wallet.omni().identity(true).output,
            bob_final_address: bob_final_address,
        };

        await omnilayer.redeemHTLC(htlcUTXO.script, bobDetails, htlcUTXO, tokenId, alpha_asset);

        const htlcBalance = await omnilayer.getBalance(tokenId, htlcUTXO.address);
        htlcBalance.should.equal("0");
        const bob_omni_balance = await omnilayer.getBalance(tokenId, bob_final_address);
        bob_omni_balance.should.equal(alpha_asset.toString());
    });

});

describe("Omni Token (USD Tether style) transferred through P2SH(P2WSH) HTLC", () => {
    const alice = actor.create("alice", {});
    const bob = actor.create("bob", {});
    const bob_final_address = "mh9g3jCJxkc4tzV88THmQHGNGiCzUZ1zg6";
    const beta_asset = 4;
    const alpha_asset = 5300;

    before(async function() {
        this.timeout(50000);
        await omnilayer.generate();
        await alice.wallet.omni().btcFund(1, true);
        await bob.wallet.omni().btcFund(1, true);
        await omnilayer.generate();
    });

    let tokenId;
    it("Create Regtest Omni Token", async function() {
        const res = await omnilayer.createOmniToken(
            "Regtest Token for HTLC",
            alice.wallet.omni().keypair,
            alice.wallet.omni().bitcoin_utxos.shift(),
            alice.wallet.omni().identity(true).output,
        );
        res.propertyid.should.be.a("number");
        tokenId = res.propertyid;
    });

    let aliceOmniUTXO;
    it("Grant Regtest Omni Token", async function() {
        const grantAmount = alpha_asset * 3;

        aliceOmniUTXO = await omnilayer.grantOmniToken(
            alice.wallet.omni().keypair,
            alice.wallet.omni().bitcoin_utxos.shift(),
            alice.wallet.omni().output,
            tokenId,
            alice.wallet.omni().identity(true).output,
            grantAmount);


        const balance = await omnilayer.getBalance(tokenId, alice.wallet.omni().identity(true).address);
        balance.should.equal(grantAmount.toString());
        const bob_omni_balance = await omnilayer.getBalance(tokenId, bob_final_address);
        bob_omni_balance.should.equal("0");
    });

    let htlcUTXO;
    it("Lock Regtest Omni Token in HTLC", async function() {
        const aliceDetails = {
            alice_keypair: alice.wallet.omni().keypair,
            alice_omni_utxo: aliceOmniUTXO,
            alice_final_address: alice.wallet.omni().identity(true).address,
        };
        const bobDetails = {
            bob_keypair: bob.wallet.omni().keypair,
            bob_btc_output: bob.wallet.omni().identity(true).output,
            bob_final_address: bob_final_address,
        };

        htlcUTXO = await omnilayer.lockInWitnessHTLC(aliceDetails, bobDetails, tokenId, alpha_asset);

        const htlcBalance = await omnilayer.getBalance(tokenId, htlcUTXO.address);
        htlcBalance.should.equal(alpha_asset.toString());
    });

    it("Redeem Regtest Omni Token from HTLC", async function() {
        const bobDetails = {
            bob_keypair: bob.wallet.omni().keypair,
            bob_btc_output: bob.wallet.omni().identity(true).output,
            bob_final_address: bob_final_address,
        };

        await omnilayer.redeemWitnessHTLC(htlcUTXO.script, htlcUTXO.p2wsh, bobDetails, htlcUTXO, tokenId, alpha_asset);

        const htlcBalance = await omnilayer.getBalance(tokenId, htlcUTXO.address);
        htlcBalance.should.equal("0");
        const bob_omni_balance = await omnilayer.getBalance(tokenId, bob_final_address);
        bob_omni_balance.should.equal(alpha_asset.toString());
    });

});
