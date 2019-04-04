import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { ActionKind, SwapRequest, SwapResponse } from "../../../lib/comit";
import { BN, toBN, toWei } from "web3-utils";
import { HarnessGlobal, sleep } from "../../../lib/util";
import { createTests } from "../../test_creator";
import chaiHttp = require("chai-http");

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

(async function() {
    const bobInitialEth = "0.1";
    const aliceInitialEth = "11";

    const alice = new Actor("alice", global.config, global.test_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });
    const bob = new Actor("bob", global.config, global.test_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });

    const bobFinalAddress = "0x03a329c0248369a73afac7f9381e02fb43d2ea72";
    const bobRefundAddress =
        "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
    const bobComitNodeAddress = bob.comitNodeConfig.comit.comit_listen;

    const alphaAssetQuantity = toBN(toWei("10", "ether"));
    const betaAssetQuantity = 100000000;
    const betaMaxFee = 5000; // Max 5000 satoshis fee

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry: number = Math.round(Date.now() / 1000) + 2; // Expires in 2 seconds

    const initialUrl = "/swaps/rfc003";
    const listUrl = "/swaps";

    await bitcoin.ensureSegwit();
    await alice.wallet.eth().fund(aliceInitialEth);
    await alice.wallet.btc().fund(0.1);
    await bob.wallet.eth().fund(bobInitialEth);
    await bob.wallet.btc().fund(10);
    await bitcoin.generate();

    let swapRequest: SwapRequest = {
        alpha_ledger: {
            name: "ethereum",
            network: "regtest",
        },
        beta_ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        alpha_asset: {
            name: "ether",
            quantity: alphaAssetQuantity.toString(),
        },
        beta_asset: {
            name: "bitcoin",
            quantity: betaAssetQuantity.toString(),
        },
        alpha_ledger_refund_identity: alice.wallet.eth().address(),
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
        peer: bobComitNodeAddress,
    };

    const actions = [
        {
            actor: bob,
            action: ActionKind.Accept,
            requestBody: {
                alpha_ledger_redeem_identity: bobFinalAddress,
            },
            state: (state: any) => state.communication.status === "ACCEPTED",
        },
        {
            actor: alice,
            action: ActionKind.Fund,
            state: (state: any) => state.alpha_ledger.status === "Funded",
        },
        {
            actor: bob,
            action: ActionKind.Fund,
            state: (state: any) => state.beta_ledger.status === "Funded",
            test: {
                description: "Waiting for beta htlc to expire",
                callback: async () => {
                    while (Date.now() / 1000 < betaExpiry + 1) {
                        await sleep(200);
                    }
                },
                timeout: 10000,
            },
        },
        {
            actor: bob,
            action: ActionKind.Refund,
            uriQuery: { address: bobRefundAddress, fee_per_byte: 20 },
        },
        {
            actor: bob,
            state: (state: any) => state.beta_ledger.status === "Refunded",
            test: {
                description:
                    "Should have received the beta asset after the refund",
                callback: async (body: any) => {
                    let refundTxId = body.state.beta_ledger.refund_tx;

                    let satoshiReceived = await bitcoin.getFirstUtxoValueTransferredTo(
                        refundTxId,
                        bobRefundAddress
                    );
                    const satoshiExpected = betaAssetQuantity - betaMaxFee;

                    satoshiReceived.should.be.at.least(satoshiExpected);
                },
            },
        },
    ];

    describe("RFC003: Ether for Bitcoin - Bitcoin (beta) refunded to Bob", () => {
        createTests(alice, bob, actions, initialUrl, listUrl, swapRequest);
    });
    run();
})();
