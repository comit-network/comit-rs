import { expect } from "chai";
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
        addressForIncomingBitcoinPayments: null,
    });

    const alphaAssetQuantity = 100000000;
    const betaAssetQuantity = ethers.utils.parseEther("10");

    const alphaExpiry = Math.round(Date.now() / 1000) + 13;
    const betaExpiry = Math.round(Date.now() / 1000) + 8;

    await bitcoin.ensureFunding();
    await bob.wallet.eth().fund("11");
    await alice.wallet.eth().fund("0.1");
    await alice.wallet.btc().fund(10);

    const swapRequest: SwapRequest = {
        alpha_ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        beta_ledger: {
            name: "ethereum",
            chain_id: 17,
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
        },
        {
            actor: alice,
            action: ActionKind.Refund,
            waitUntil: state => state.alpha_ledger.status === "REFUNDED",
        },
        {
            actor: bob,
            waitUntil: state => state.alpha_ledger.status === "REFUNDED",
        },
        {
            actor: alice,
            test: {
                description: "Should see that beta is still funded",
                callback: async body => {
                    const status = body.properties.state.beta_ledger.status;

                    expect(status).to.equal("FUNDED");
                },
            },
        },
        {
            actor: bob,
            test: {
                description: "Should see that beta is still funded",
                callback: async body => {
                    const status = body.properties.state.beta_ledger.status;

                    expect(status).to.equal("FUNDED");
                },
            },
        },
        {
            actor: bob,
            action: ActionKind.Refund,
            waitUntil: state => state.beta_ledger.status === "REFUNDED",
        },
        {
            actor: alice,
            waitUntil: state => state.beta_ledger.status === "REFUNDED",
        },
    ];

    describe("RFC003: Alice can refund before Bob", async () => {
        createTests(alice, bob, steps, "/swaps/rfc003", "/swaps", swapRequest);
    });
    run();
})();
