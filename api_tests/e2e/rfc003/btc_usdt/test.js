const chai = require("chai");
chai.use(require("chai-http"));
const omnilayer = require("../../../lib/omnilayer.js");
const actor = require("../../../lib/actor.js");
const should = chai.should();
const logger = global.harness.logger;

const omni_rpc_client = omnilayer.create_client();

const alice = actor.create("alice", {});
const bob = actor.create("bob", {});

const alice_final_address = "mxHrqVGroA6VgNeZR7ndjFkywNozVdYEYT";
const bob_final_address =
    "mzNFGtxdTSTJ1Lh6fq5N5oUgbhwA7Nm7cA";

const alpha_asset = 100000000;
const beta_asset = 5000;
const alpha_max_fee = 5000; // Max 5000 satoshis fee

const alpha_expiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
const beta_expiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

describe("RFC003: Bitcoin for USD Tether (Omnilayer)", () => {
    before(async function() {
        this.timeout(50000);
        //await omnilayer.activate_segwit();
        await alice.wallet.omni().omniFund(1);
    });

    it("Create RegtestOmniCoin", async function() {
      const res = await alice.wallet.omni().createPayloadIssuanceManaged();
      console.log(res);
    });
});
