import "chai/register-should";
import { ethers } from "ethers";
import { Actor } from "../../../lib/actor";
import * as bitcoin from "../../../lib/bitcoin";
import { ActionKind, SwapRequest } from "../../../lib/comit";
import "../../../lib/setup_chai";
import { createTests, Step } from "../../../lib/test_creator";
import { HarnessGlobal } from "../../../lib/util";

declare var global: HarnessGlobal;

(async function() {
    const alice = new Actor("alice", {
        ledgerConfig: global.ledgerConfigs,
        addressForIncomingBitcoinPayments:
            "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0",
    });
    const bob = new Actor("bob", {
        ledgerConfig: global.ledgerConfigs,
    });

    const alphaAssetQuantity = 100000000;
    const betaAssetQuantity = ethers.utils.parseEther("10");
    const maxFeeInSatoshi = 50000;

    const alphaExpiry = Math.round(Date.now() / 1000) + 13;
    const betaExpiry = Math.round(Date.now() / 1000) + 8;

    await bitcoin.ensureFunding();
    await bob.wallet.eth().fund("11");
    await alice.wallet.eth().fund("0.1");
    await alice.wallet.btc().fund(10);

    const bobInitialWei = await bob.wallet.eth().ethBalance();

    const swapRequest: SwapRequest = {
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
        beta_ledger_redeem_identity: alice.wallet.eth().address(),
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
        peer: await bob.peerId(),
    };

    const steps: Step[] = [
        {
            actor: bob,
            action: ActionKind.Accept,
            waitUntil: state => state.communication.status === "ACCEPTED",
        },
        {
            actor: alice,
            action: ActionKind.Fund,
            waitUntil: state => state.alpha_ledger.status === "FUNDED",
        },
        {
            actor: bob,
            action: ActionKind.Fund,
            waitUntil: state =>
                state.alpha_ledger.status === "FUNDED" &&
                state.beta_ledger.status === "FUNDED",
            test: {
                description: "Should have less beta asset after the funding",
                callback: async () => {
                    const bobWeiBalanceAfter = await bob.wallet
                        .eth()
                        .ethBalance();

                    bobWeiBalanceAfter.lt(bobInitialWei).should.be.equal(true);
                },
            },
        },
        {
            actor: bob,
            action: ActionKind.Refund,
            waitUntil: state => state.beta_ledger.status === "REFUNDED",
            test: {
                description:
                    "Should have received the beta asset after the refund",
                callback: async () => {
                    const bobWeiBalanceAfter = await bob.wallet
                        .eth()
                        .ethBalance();

                    bobWeiBalanceAfter.eq(bobInitialWei).should.be.equal(true);
                },
            },
        },
        {
            actor: alice,
            action: ActionKind.Refund,
        },
        {
            actor: alice,
            waitUntil: state =>
                state.alpha_ledger.status === "REFUNDED" &&
                state.beta_ledger.status === "REFUNDED",
            test: {
                description:
                    "Should have received the alpha asset after the refund",
                callback: async body => {
                    const refundTxId =
                        body.properties.state.alpha_ledger.refund_tx;

                    const satoshiReceived = await alice.wallet
                        .btc()
                        .satoshiReceivedInTx(refundTxId);
                    const satoshiExpected =
                        alphaAssetQuantity - maxFeeInSatoshi;

                    satoshiReceived.should.be.at.least(satoshiExpected);
                },
            },
        },
    ];

    describe("RFC003: Bitcoin for Ether - Both refunded", async () => {
        createTests(alice, bob, steps, "/swaps/rfc003", "/swaps", swapRequest);
    });
    run();
})();
