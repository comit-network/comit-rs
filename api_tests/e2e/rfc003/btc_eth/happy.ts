import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import { Actor } from "../../../lib/actor";
import { AcceptPayload, SwapRequest, SwapResponse } from "../../../lib/comit";
import { toBN, toWei, BN } from "web3-utils";
import { HarnessGlobal } from "../../../lib/util";
import { ActionTrigger, AfterTest, execute, Method } from "../../test_executor";
import chaiHttp = require("chai-http");
import * as ethereum from "../../../lib/ethereum";

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

const bob_initial_eth = "11";
const alice_initial_eth = "0.1";

const alice = new Actor("alice", global.config, global.test_root, {
    ethConfig: global.ledgers_config.ethereum,
    btcConfig: global.ledgers_config.bitcoin,
});
const bob = new Actor("bob", global.config, global.test_root, {
    ethConfig: global.ledgers_config.ethereum,
    btcConfig: global.ledgers_config.bitcoin,
});

const aliceFinalAddress = "0x03a329c0248369a73afac7f9381e02fb43d2ea72";
const bobFinalAddress =
    "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
const bobComitNodeListen = bob.comitNodeConfig.comit.comit_listen;

const alphaAssetQuantity = 100000000;
const betaAssetQuantity = toBN(toWei("10", "ether"));
const alphaMaxFee = 5000; // Max 5000 satoshis fee

const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

let swapRequest: SwapRequest = {
    alpha_ledger: {
        name: "Bitcoin",
        network: "regtest",
    },
    beta_ledger: {
        name: "Ethereum",
        network: "regtest",
    },
    alpha_asset: {
        name: "Bitcoin",
        quantity: alphaAssetQuantity.toString(),
    },
    beta_asset: {
        name: "Ether",
        quantity: betaAssetQuantity.toString(),
    },
    beta_ledger_redeem_identity: aliceFinalAddress,
    alpha_expiry: alphaExpiry,
    beta_expiry: betaExpiry,
    peer: bobComitNodeListen,
};

let bobAcceptPayload: AcceptPayload = {
    beta_ledger_refund_identity: bob.wallet.eth().address(),
    alpha_ledger_redeem_identity: null,
};

const aliceRedeemTest = new AfterTest(
    "[alice] Should have received the beta asset after the redeem",
    async function(swapLocations: { [key: string]: string }) {
        let body = (await alice.pollComitNodeUntil(
            swapLocations["alice"],
            body => body.state.beta_ledger.status === "Redeemed"
        )) as SwapResponse;

        const aliceEthBalanceAfter = await ethereum.ethBalance(
            aliceFinalAddress
        );
        const aliceEthBalanceExpected = aliceEthBalanceBefore.add(
            betaAssetQuantity
        );
        aliceEthBalanceAfter.eq(aliceEthBalanceExpected).should.be.equal(true);
    }
);

const bobRedeemTest = new AfterTest(
    "[bob] Should have received the alpha asset after the redeem",
    async function(swapLocations: { [key: string]: string }) {
        let body = (await bob.pollComitNodeUntil(
            swapLocations["bob"],
            body => body.state.alpha_ledger.status === "Redeemed"
        )) as SwapResponse;
        let redeemTxId = body.state.alpha_ledger.redeem_tx;

        let satoshiReceived = await bitcoin.getFirstUtxoValueTransferredTo(
            redeemTxId,
            bobFinalAddress
        );
        const satoshiExpected = alphaAssetQuantity - alphaMaxFee;

        satoshiReceived.should.be.at.least(satoshiExpected);
    },
    10000
);

const actions: ActionTrigger[] = [
    new ActionTrigger({
        actor: bob,
        name: "accept",
        method: Method.Post,
        timeout: 10000,
        payload: bobAcceptPayload,
    }),
    new ActionTrigger({
        actor: alice,
        name: "fund",
        method: Method.Get,
        timeout: 10000,
    }),
    new ActionTrigger({
        actor: bob,
        name: "fund",
        method: Method.Get,
        timeout: 10000,
    }),
    new ActionTrigger({
        actor: alice,
        name: "redeem",
        method: Method.Get,
        timeout: 10000,
        afterTest: aliceRedeemTest,
    }),
    new ActionTrigger({
        actor: bob,
        name: "redeem",
        method: Method.Get,
        timeout: 10000,
        parameters: "address=" + bobFinalAddress + "&fee_per_byte=20",
        afterTest: bobRedeemTest,
    }),
];

const initialUrl = "/swaps/rfc003";
const listUrl = "/swaps";

let aliceEthBalanceBefore: BN;

describe("RFC003: Bitcoin for Ether", async () => {
    before(async function() {
        this.timeout(5000);
        await bitcoin.ensureSegwit();
        await bob.wallet.eth().fund(bob_initial_eth);
        await alice.wallet.eth().fund(alice_initial_eth);
        await alice.wallet.btc().fund(10);
        await bitcoin.generate();
        aliceEthBalanceBefore = await ethereum.ethBalance(aliceFinalAddress);
    });

    await execute(alice, bob, actions, initialUrl, listUrl, swapRequest);
});
