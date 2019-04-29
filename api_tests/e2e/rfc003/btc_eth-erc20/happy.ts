import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { ActionKind, SwapRequest, SwapResponse } from "../../../lib/comit";
import { Wallet } from "../../../lib/wallet";
import { BN, toBN, toWei } from "web3-utils";
import { HarnessGlobal } from "../../../lib/util";
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
    const bobInitialEth = "5";
    const bobInitialErc20 = toBN(toWei("10000", "ether"));

    const alice = new Actor("alice", global.config, global.project_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });
    const bob = new Actor("bob", global.config, global.project_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });

    const aliceFinalAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const bobFinalAddress =
        "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
    const bobComitNodeAddress = bob.comitNodeConfig.comit.comit_listen;

    const alphaAssetQuantity = 100000000;
    const betaAssetQuantity = toBN(toWei("5000", "ether"));
    const alphaMaxFee = 5000; // Max 5000 satoshis fee

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    const initialUrl = "/swaps/rfc003";
    const listUrl = "/swaps";

    await bitcoin.ensureFunding();
    await tobyWallet.eth().fund(tobyInitialEth);
    await bob.wallet.eth().fund(bobInitialEth);
    await alice.wallet.btc().fund(10);
    await bitcoin.generate();
    await alice.wallet.eth().fund("1");

    let deployReceipt = await tobyWallet
        .eth()
        .deployErc20TokenContract(global.project_root);
    let tokenContractAddress: string = deployReceipt.contractAddress;

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
            name: "erc20",
            quantity: betaAssetQuantity.toString(),
            token_contract: tokenContractAddress,
        },
        beta_ledger_redeem_identity: aliceFinalAddress,
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
        peer: bobComitNodeAddress,
    };

    let bobWalletAddress = await bob.wallet.eth().address();

    let mintReceipt = await ethereum.mintErc20Tokens(
        tobyWallet.eth(),
        tokenContractAddress,
        bobWalletAddress,
        bobInitialErc20
    );
    mintReceipt.status.should.equal(true);

    let erc20Balance = await ethereum.erc20Balance(
        bobWalletAddress,
        tokenContractAddress
    );

    erc20Balance.eq(bobInitialErc20).should.equal(true);

    let aliceErc20BalanceBefore: BN = await ethereum.erc20Balance(
        aliceFinalAddress,
        tokenContractAddress
    );

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
            action: ActionKind.Deploy,
            state: (state: any) => state.beta_ledger.status === "Deployed",
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
                    let aliceErc20BalanceAfter = await ethereum.erc20Balance(
                        aliceFinalAddress,
                        tokenContractAddress
                    );

                    let aliceErc20BalanceExpected = aliceErc20BalanceBefore.add(
                        betaAssetQuantity
                    );
                    aliceErc20BalanceAfter
                        .eq(aliceErc20BalanceExpected)
                        .should.equal(true);
                },
                timeout: 5000,
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

    describe("RFC003: Bitcoin for ERC20", async () => {
        createTests(alice, bob, actions, initialUrl, listUrl, swapRequest);
    });
    run();
})();
