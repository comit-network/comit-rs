import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { Action, SwapRequest, SwapResponse } from "../../../lib/comit";
import { Wallet } from "../../../lib/wallet";
import { BN, toBN, toWei } from "web3-utils";
import { HarnessGlobal } from "../../../lib/util";
import { ActionTrigger, AfterTest, execute } from "../../test_executor";
import chaiHttp = require("chai-http");

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

async function test() {
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

    const aliceFinalAddress =
        "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
    const bobFinalAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const bobComitNodeAddress = bob.comitNodeConfig.comit.comit_listen;
    const alphaAssetQuantity = toBN(toWei("5000", "ether"));

    const betaAssetQuantity = 100000000;
    const betaMaxFee = 5000; // Max 5000 satoshis fee

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

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
        .deployErc20TokeContract(global.project_root);
    let tokenContractAddress: string = deployReceipt.contractAddress;

    let swapRequest: SwapRequest = {
        alpha_ledger: {
            name: "Ethereum",
            network: "regtest",
        },
        beta_ledger: {
            name: "Bitcoin",
            network: "regtest",
        },
        alpha_asset: {
            name: "ERC20",
            quantity: alphaAssetQuantity.toString(),
            token_contract: tokenContractAddress,
        },
        beta_asset: {
            name: "Bitcoin",
            quantity: betaAssetQuantity.toString(),
        },
        alpha_ledger_refund_identity: bobFinalAddress,
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
        peer: bobComitNodeAddress,
    };

    let aliceWalletAddress = await alice.wallet.eth().address();

    let mintReceipt = await ethereum.mintErc20Tokens(
        tobyWallet.eth(),
        tokenContractAddress,
        aliceWalletAddress,
        aliceInitialErc20
    );
    mintReceipt.status.should.equal(true);

    let erc20Balance = await ethereum.erc20Balance(
        aliceWalletAddress,
        tokenContractAddress
    );

    erc20Balance.eq(aliceInitialErc20).should.equal(true);

    let bobErc20BalanceBefore: BN = await ethereum.erc20Balance(
        bobFinalAddress,
        tokenContractAddress
    );

    const actions: ActionTrigger[] = [
        new ActionTrigger({
            actor: bob,
            action: Action.Accept,
            timeout: 10000,
            payload: {
                beta_ledger_refund_identity: bob.wallet.eth().address(),
                alpha_ledger_redeem_identity: bobFinalAddress,
            },
        }),
        new ActionTrigger({
            actor: alice,
            action: Action.Deploy,
            timeout: 10000,
        }),
        new ActionTrigger({
            actor: alice,
            action: Action.Fund,
            timeout: 10000,
        }),
        new ActionTrigger({
            actor: bob,
            action: Action.Fund,
            timeout: 10000,
        }),
        new ActionTrigger({
            actor: alice,
            action: Action.Redeem,
            timeout: 10000,
            parameters: "address=" + aliceFinalAddress + "&fee_per_byte=20",
            afterTest: new AfterTest(
                "[alice] Should have received the beta asset after the redeem",
                async function(swapLocations: { [key: string]: string }) {
                    let body = (await alice.pollComitNodeUntil(
                        swapLocations["alice"],
                        body => body.state.beta_ledger.status === "Redeemed"
                    )) as SwapResponse;
                    let redeemTxId = body.state.beta_ledger.redeem_tx;

                    let satoshiReceived = await bitcoin.getFirstUtxoValueTransferredTo(
                        redeemTxId,
                        aliceFinalAddress
                    );
                    const satoshiExpected = betaAssetQuantity - betaMaxFee;

                    satoshiReceived.should.be.at.least(satoshiExpected);
                },
                5000
            ),
        }),
        new ActionTrigger({
            actor: bob,
            action: Action.Redeem,
            timeout: 10000,
            afterTest: new AfterTest(
                "[bob] Should have received the alpha asset after the redeem",
                async function() {
                    let erc20BalanceAfter = await ethereum.erc20Balance(
                        bobFinalAddress,
                        tokenContractAddress
                    );

                    let erc20BalanceExpected = bobErc20BalanceBefore.add(
                        alphaAssetQuantity
                    );

                    erc20BalanceAfter
                        .eq(erc20BalanceExpected)
                        .should.be.equal(true);
                },
                10000
            ),
        }),
    ];

    describe("RFC003: ERC20 for Bitcoin", () => {
        execute(alice, bob, actions, initialUrl, listUrl, swapRequest);
    });
    run();
}

test();
