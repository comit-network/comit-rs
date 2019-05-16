import * as bitcoin from "../../../lib/bitcoin";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { ActionKind, SwapRequest } from "../../../lib/comit";
import { Wallet } from "../../../lib/wallet";
import { BN, toBN, toWei } from "web3-utils";
import { HarnessGlobal } from "../../../lib/util";
import { ActionTrigger, createTests } from "../../test_creator";
import "chai/register-should";
import "../../../lib/setupChai";

declare var global: HarnessGlobal;

(async function() {
    const tobyWallet = new Wallet("toby", {
        ethereumNodeConfig: global.ledgers_config.ethereum,
    });
    const alice = new Actor("alice", global.config, global.project_root, {
        ethereumNodeConfig: global.ledgers_config.ethereum,
        bitcoinNodeConfig: global.ledgers_config.bitcoin,
        addressForIncomingBitcoinPayments:
            "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0",
    });
    const bob = new Actor("bob", global.config, global.project_root, {
        ethereumNodeConfig: global.ledgers_config.ethereum,
        bitcoinNodeConfig: global.ledgers_config.bitcoin,
    });

    const alphaAssetQuantity = toBN(toWei("5000", "ether"));

    const betaAssetQuantity = 100000000;
    const maxFeeInSatoshi = 5000;

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    await bitcoin.ensureFunding();
    await tobyWallet.eth().fund("10");
    await alice.wallet.eth().fund("5");
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
        peer: await bob.peerId(),
    };

    let aliceWalletAddress = await alice.wallet.eth().address();

    let mintReceipt = await ethereum.mintErc20Tokens(
        tobyWallet.eth(),
        tokenContractAddress,
        aliceWalletAddress,
        toBN(toWei("10000", "ether"))
    );
    mintReceipt.status.should.equal(true);

    let erc20Balance = await ethereum.erc20Balance(
        aliceWalletAddress,
        tokenContractAddress
    );

    erc20Balance.eq(toBN(toWei("10000", "ether"))).should.equal(true);

    let bobErc20BalanceBefore: BN = await bob.wallet
        .eth()
        .erc20Balance(tokenContractAddress);

    const actions: ActionTrigger[] = [
        {
            actor: bob,
            action: ActionKind.Accept,
            state: state => state.communication.status === "ACCEPTED",
        },
        {
            actor: alice,
            action: ActionKind.Deploy,
            state: state => state.alpha_ledger.status === "Deployed",
        },
        {
            actor: alice,
            action: ActionKind.Fund,
            state: state => state.alpha_ledger.status === "Funded",
        },
        {
            actor: bob,
            action: ActionKind.Fund,
            state: state => state.beta_ledger.status === "Funded",
        },
        {
            actor: alice,
            action: ActionKind.Redeem,
            state: state => state.beta_ledger.status === "Redeemed",
            test: {
                description:
                    "Should have received the beta asset after the redeem",
                callback: async body => {
                    let redeemTxId =
                        body.properties.state.beta_ledger.redeem_tx;

                    let satoshiReceived = await alice.wallet
                        .btc()
                        .moneyReceivedInTx(redeemTxId);
                    const satoshiExpected = betaAssetQuantity - maxFeeInSatoshi;

                    satoshiReceived.should.be.at.least(satoshiExpected);
                },
                timeoutOverride: 5000,
            },
        },
        {
            actor: bob,
            action: ActionKind.Redeem,
            state: state => state.alpha_ledger.status === "Redeemed",
            test: {
                description:
                    "Should have received the alpha asset after the redeem",
                callback: async () => {
                    let erc20BalanceAfter = await bob.wallet
                        .eth()
                        .erc20Balance(tokenContractAddress);

                    let erc20BalanceExpected = bobErc20BalanceBefore.add(
                        alphaAssetQuantity
                    );

                    erc20BalanceAfter
                        .eq(erc20BalanceExpected)
                        .should.be.equal(true);
                },
                timeoutOverride: 10000,
            },
        },
    ];

    describe("RFC003: ERC20 for Bitcoin", () => {
        createTests(
            alice,
            bob,
            actions,
            "/swaps/rfc003",
            "/swaps",
            swapRequest
        );
    });
    run();
})();
