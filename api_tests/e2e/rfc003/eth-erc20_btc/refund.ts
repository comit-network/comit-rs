import "chai/register-should";
import { ethers } from "ethers";
import { Actor } from "../../../lib/actor";
import * as bitcoin from "../../../lib/bitcoin";
import { ActionKind, SwapRequest } from "../../../lib/comit";
import "../../../lib/setup_chai";
import { createTests, Step } from "../../../lib/test_creator";
import { HarnessGlobal } from "../../../lib/util";
import { Wallet } from "../../../lib/wallet";

declare var global: HarnessGlobal;

(async function() {
    const tobyWallet = new Wallet("toby", {
        ledgerConfig: global.ledgerConfigs,
    });
    const alice = new Actor("alice", {
        ledgerConfig: global.ledgerConfigs,
    });
    const bob = new Actor("bob", {
        ledgerConfig: global.ledgerConfigs,
        addressForIncomingBitcoinPayments:
            "bcrt1qc45uezve8vj8nds7ws0da8vfkpanqfxecem3xl7wcs3cdne0358q9zx9qg",
    });

    const aliceInitialErc20 = ethers.utils.parseEther("10000");
    const alphaAssetQuantity = ethers.utils.parseEther("5000");
    const betaAssetQuantity = 100000000;
    const maxFeeInSatoshi = 50000;

    const alphaExpiry = Math.round(Date.now() / 1000) + 13;
    const betaExpiry = Math.round(Date.now() / 1000) + 8;

    await bitcoin.ensureFunding();
    await tobyWallet.eth().fund("10");
    await alice.wallet.eth().fund("5");
    await bob.wallet.btc().fund(10);
    await bob.wallet.eth().fund("1");

    const tokenContractAddress = await tobyWallet
        .eth()
        .deployErc20TokenContract(global.projectRoot);
    await tobyWallet
        .eth()
        .mintErc20To(
            alice.wallet.eth().address(),
            aliceInitialErc20,
            tokenContractAddress
        );

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
            name: "erc20",
            quantity: alphaAssetQuantity.toString(),
            token_contract: tokenContractAddress,
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

    const erc20Balance = await alice.wallet
        .eth()
        .erc20Balance(tokenContractAddress);
    erc20Balance.eq(aliceInitialErc20).should.equal(true);

    const steps: Step[] = [
        {
            actor: bob,
            action: ActionKind.Accept,
            waitUntil: state => state.communication.status === "ACCEPTED",
        },
        {
            actor: alice,
            action: ActionKind.Deploy,
            waitUntil: state => state.alpha_ledger.status === "DEPLOYED",
        },
        {
            actor: alice,
            action: ActionKind.Fund,
            waitUntil: state => state.alpha_ledger.status === "FUNDED",
            test: {
                description: "Should have less alpha asset after the funding",
                callback: async () => {
                    const erc20BalanceAfter = await alice.wallet
                        .eth()
                        .erc20Balance(tokenContractAddress);
                    erc20BalanceAfter
                        .lt(aliceInitialErc20)
                        .should.be.equal(true);
                },
            },
        },
        {
            actor: bob,
            action: ActionKind.Fund,
            waitUntil: state =>
                state.alpha_ledger.status === "FUNDED" &&
                state.beta_ledger.status === "FUNDED",
        },
        {
            actor: bob,
            action: ActionKind.Refund,
            waitUntil: state => state.beta_ledger.status === "REFUNDED",
            test: {
                description:
                    "Should have received the beta asset after the refund",
                callback: async body => {
                    const refundTxId =
                        body.properties.state.beta_ledger.refund_tx;

                    const satoshiReceived = await bob.wallet
                        .btc()
                        .satoshiReceivedInTx(refundTxId);
                    const satoshiExpected = betaAssetQuantity - maxFeeInSatoshi;

                    satoshiReceived.should.be.at.least(satoshiExpected);
                },
            },
        },
        {
            actor: alice,
            action: ActionKind.Refund,
            waitUntil: state => state.alpha_ledger.status === "REFUNDED",
            test: {
                description:
                    "Should have received the alpha asset after the refund",
                callback: async () => {
                    const erc20BalanceAfter = await alice.wallet
                        .eth()
                        .erc20Balance(tokenContractAddress);
                    erc20BalanceAfter
                        .eq(aliceInitialErc20)
                        .should.be.equal(true);
                },
            },
        },
    ];

    describe("RFC003: Ether for ERC20 - Both refunded", async () => {
        createTests(alice, bob, steps, "/swaps/rfc003", "/swaps", swapRequest);
    });
    run();
})();
