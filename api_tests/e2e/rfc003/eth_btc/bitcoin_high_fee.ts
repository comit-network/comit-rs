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
    const alice = new Actor(
        "alice",
        {
            ledgerConfig: global.ledgerConfigs,
            addressForIncomingBitcoinPayments:
                "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0",
        },
        null,
        {
            bitcoinFeePerWU: 100000000,
        }
    );
    const bob = new Actor(
        "bob",
        {
            ledgerConfig: global.ledgerConfigs,
            addressForIncomingBitcoinPayments:
                "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0",
        },
        null,
        {
            bitcoinFeePerWU: 100000000,
        }
    );

    const alphaAssetQuantity = ethers.utils.parseEther("10");
    const betaAssetQuantity = 100000000;

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    await bitcoin.ensureFunding();
    await alice.wallet.eth().fund("11");
    await alice.wallet.btc().fund(0.1);
    await bob.wallet.eth().fund("0.1");
    await bob.wallet.btc().fund(10);

    const swapRequest: SwapRequest = {
        alpha_ledger: {
            name: "ethereum",
            chain_id: 17,
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
            waitUntil: state => state.beta_ledger.status === "FUNDED",
        },
        {
            actor: alice,
            action: {
                kind: ActionKind.Redeem,
                test: response => {
                    expect(response).to.have.status(400);
                    expect(response.body.title).to.equal("Fee is too high.");
                },
            },
        },
        {
            actor: bob,
            action: {
                kind: ActionKind.Refund,
                test: response => {
                    expect(response).to.have.status(400);
                    expect(response.body.title).to.equal("Fee is too high.");
                },
            },
        },
    ];

    describe("RFC003: Ether for Bitcoin - Redeem/Refund Bitcoin with high fee", () => {
        createTests(alice, bob, steps, "/swaps/rfc003", "/swaps", swapRequest);
    });
    run();
})();
