import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { ActionKind, SwapRequest } from "../../../lib/comit";
import { BN, toBN, toWei } from "web3-utils";
import { HarnessGlobal } from "../../../lib/util";
import { createTests } from "../../test_creator";
import chaiHttp = require("chai-http");

chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

(async function() {
    const bobInitialEth = "11";
    const aliceInitialEth = "0.1";

    const alice = new Actor("alice", global.config, global.project_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });
    const bob = new Actor("bob", global.config, global.project_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });

    const aliceFinalAddress = "0x03a329c0248369a73afac7f9381e02fb43d2ea72";
    const bobFinalAddress =
        "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
    const bobComitNode = await bob.peerId();

    const alphaAssetQuantity = 100000000;
    const betaAssetQuantity = toBN(toWei("10", "ether"));
    const alphaMaxFee = 5000; // Max 5000 satoshis fee

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    const initialUrl = "/swaps/rfc003";
    const listUrl = "/swaps";

    await bitcoin.ensureFunding();
    await bob.wallet.eth().fund(bobInitialEth);
    await alice.wallet.eth().fund(aliceInitialEth);
    await alice.wallet.btc().fund(10);
    await bitcoin.generate();
    let aliceEthBalanceBefore: BN = await ethereum.ethBalance(
        aliceFinalAddress
    );

    let swapRequest: SwapRequest = {
        alpha_ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        beta_ledger: {
            name: "ethereum",
            network: "regtest",
        },
        alpha_asset: {
            name: "bitcoin",
            quantity: alphaAssetQuantity.toString(),
        },
        beta_asset: {
            name: "ether",
            quantity: betaAssetQuantity.toString(),
        },
        beta_ledger_redeem_identity: aliceFinalAddress,
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
        peer: bobComitNode,
    };

    const actions = [
        {
            actor: bob,
            action: ActionKind.Accept,
            requestBody: {
                beta_ledger_refund_identity: bob.wallet.eth().address(),
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
        },
        {
            actor: alice,
            action: ActionKind.Redeem,
            state: (state: any) => state.beta_ledger.status === "Redeemed",
            test: {
                description:
                    "Should have received the beta asset after the redeem",
                callback: async () => {
                    const aliceEthBalanceAfter = await ethereum.ethBalance(
                        aliceFinalAddress
                    );
                    const aliceEthBalanceExpected = aliceEthBalanceBefore.add(
                        betaAssetQuantity
                    );
                    aliceEthBalanceAfter
                        .eq(aliceEthBalanceExpected)
                        .should.be.equal(true);
                },
            },
        },
        {
            actor: bob,
            action: ActionKind.Redeem,
            uriQuery: { address: bobFinalAddress, fee_per_byte: 20 },
            state: (state: any) => state.alpha_ledger.status === "Redeemed",
            test: {
                description:
                    "Should have received the alpha asset after the redeem",
                callback: async (body: any) => {
                    let redeemTxId = body.state.alpha_ledger.redeem_tx;

                    let satoshiReceived = await bitcoin.getFirstUtxoValueTransferredTo(
                        redeemTxId,
                        bobFinalAddress
                    );
                    const satoshiExpected = alphaAssetQuantity - alphaMaxFee;

                    satoshiReceived.should.be.at.least(satoshiExpected);
                },
                timeout: 10000,
            },
        },
    ];

    describe("RFC003: Bitcoin for Ether", async () => {
        createTests(alice, bob, actions, initialUrl, listUrl, swapRequest);
    });
    run();
})();
