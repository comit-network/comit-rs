import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { ActionKind, SwapRequest, SwapResponse } from "../../../lib/comit";
import { Wallet } from "../../../lib/wallet";
import { BN, toBN, toWei } from "web3-utils";
import { HarnessGlobal, sleep } from "../../../lib/util";
import { createTests } from "../../test_creator";
import chaiHttp = require("chai-http");

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

(async function() {
    const tobyWallet = new Wallet("toby", {
        ethConfig: global.ledgers_config.ethereum,
    });

    const tobyInitialEth = "10";
    const aliceInitialEth = "5";
    const aliceInitialErc20 = toBN(toWei("10000", "ether"));

    const alice = new Actor("alice", global.config, global.test_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });
    const bob = new Actor("bob", global.config, global.test_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });

    const bobRefundAddress =
        "bcrt1qc45uezve8vj8nds7ws0da8vfkpanqfxecem3xl7wcs3cdne0358q9zx9qg";
    const bobFinalAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const bobComitNodeAddress = bob.comitNodeConfig.comit.comit_listen;
    const alphaAssetQuantity = toBN(toWei("5000", "ether"));

    const betaAssetQuantity = 100000000;
    const betaMaxFee = 5000; // Max 5000 satoshis fee

    const alphaExpiry: number = Math.round(Date.now() / 1000) + 3; // Expires in 3 seconds
    const betaExpiry: number = Math.round(Date.now() / 1000) + 1; // Expires in 1 seconds

    const initialUrl = "/swaps/rfc003";
    const listUrl = "/swaps";

    await bitcoin.ensureSegwit();
    await tobyWallet.eth().fund(tobyInitialEth);
    await alice.wallet.eth().fund(aliceInitialEth);
    await bob.wallet.btc().fund(10);
    await bitcoin.generate();
    await bob.wallet.eth().fund("1");

    let deployReceipt = await tobyWallet
        .eth()
        .deployErc20TokenContract(global.project_root);
    let tokenContractAddress: string = deployReceipt.contractAddress;

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
        peer: bobComitNodeAddress,
    };

    let mintReceipt = await ethereum.mintErc20Tokens(
        tobyWallet.eth(),
        tokenContractAddress,
        alice.wallet.eth().address(),
        aliceInitialErc20
    );
    mintReceipt.status.should.equal(true);

    let erc20Balance = await ethereum.erc20Balance(
        alice.wallet.eth().address(),
        tokenContractAddress
    );

    erc20Balance.eq(aliceInitialErc20).should.equal(true);

    const actions = [
        {
            actor: bob,
            action: ActionKind.Accept,
            requestBody: {
                beta_ledger_refund_identity: bob.wallet.eth().address(),
                alpha_ledger_redeem_identity: bobFinalAddress,
            },
            state: (state: any) => state.communication.status === "ACCEPTED",
        },
        {
            actor: alice,
            action: ActionKind.Deploy,
            state: (state: any) => state.alpha_ledger.status === "Deployed",
        },
        {
            actor: alice,
            action: ActionKind.Fund,
            state: (state: any) => state.alpha_ledger.status === "Funded",
            test: {
                description:
                    "[alice] Should have less alpha asset after the funding",
                callback: async () => {
                    let erc20BalanceAfter = await ethereum.erc20Balance(
                        alice.wallet.eth().address(),
                        tokenContractAddress
                    );
                    erc20BalanceAfter
                        .lt(aliceInitialErc20)
                        .should.be.equal(true);
                },
            },
        },
        {
            actor: bob,
            action: ActionKind.Fund,
            state: (state: any) =>
                state.alpha_ledger.status === "Funded" &&
                state.beta_ledger.status === "Funded",
        },
        {
            actor: bob,
            test: {
                description: "[bob] Is waiting for beta htlc to expire",
                callback: async () => {
                    while (Date.now() / 1000 < betaExpiry) {
                        await sleep(200);
                    }
                },
            },
        },
        {
            actor: bob,
            action: ActionKind.Refund,
            uriQuery: { address: bobRefundAddress, fee_per_byte: 20 },
            state: (state: any) => state.beta_ledger.status === "Refunded",
            test: {
                description:
                    "[bob] Should have received the beta asset after the refund",
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
        {
            actor: alice,
            test: {
                description: "[alice] Is waiting for alpha htlc to expire",
                callback: async () => {
                    while (Date.now() / 1000 < alphaExpiry) {
                        await sleep(200);
                    }
                },
            },
        },
        {
            actor: alice,
            action: ActionKind.Refund,
            state: (state: any) => state.alpha_ledger.status === "Refunded",
            test: {
                description:
                    "[alice] Should have received the alpha asset after the refund",
                callback: async () => {
                    let erc20BalanceAfter = await ethereum.erc20Balance(
                        alice.wallet.eth().address(),
                        tokenContractAddress
                    );
                    erc20BalanceAfter
                        .eq(aliceInitialErc20)
                        .should.be.equal(true);
                },
                timeout: 10000,
            },
            timeout: 10000,
        },
    ];

    describe("RFC003: Ether for ERC20 - Both refunded", async () => {
        createTests(alice, bob, actions, initialUrl, listUrl, swapRequest);
    });
    run();
})();
