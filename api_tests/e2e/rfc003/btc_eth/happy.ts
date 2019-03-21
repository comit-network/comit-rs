import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { ActionKind, SwapRequest, SwapResponse } from "../../../lib/comit";
import { BN, toBN, toWei } from "web3-utils";
import { HarnessGlobal } from "../../../lib/util";
import { createTests } from "../../test_creator";
import chaiHttp = require("chai-http");

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

(async function() {
    const bobInitialEth = "11";
    const aliceInitialEth = "0.1";

    const alice = new Actor("alice", global.config, global.test_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });
    const bob = new Actor("bob", global.config, global.test_root, {
        ethConfig: global.ledgers_config.ethereum,
        btcConfig: global.ledgers_config.bitcoin,
    });

    const aliceFinalAddress = "0x03a329c0248369a73afac7f9381e02fb43d2ea72";
    const bobFinalAddress =
        "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0";
    const bobComitNodeListen = bob.comitNodeConfig.comit.comit_listen;

    const alphaAssetQuantity = 100000000;
    const betaAssetQuantity = toBN(toWei("10", "ether"));
    const alphaMaxFee = 5000; // Max 5000 satoshis fee

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    const initialUrl = "/swaps/rfc003";
    const listUrl = "/swaps";

    await bitcoin.ensureSegwit();
    await bob.wallet.eth().fund(bobInitialEth);
    await alice.wallet.eth().fund(aliceInitialEth);
    await alice.wallet.btc().fund(10);
    await bitcoin.generate();
    let aliceEthBalanceBefore: BN = await ethereum.ethBalance(
        aliceFinalAddress
    );

    let swapRequest: SwapRequest = {
        alpha_ledger: {
            name: "Bitcoin",
            network: "regtest",
        },
        beta_ledger: {
            name: "Ethereum",
            network: "regtest",
        },
        alpha_asset: {
            name: "Bitcoin",
            quantity: alphaAssetQuantity.toString(),
        },
        beta_asset: {
            name: "Ether",
            quantity: betaAssetQuantity.toString(),
        },
        beta_ledger_redeem_identity: aliceFinalAddress,
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
        peer: bobComitNodeListen,
    };

    const actions = [
        {
            actor: bob,
            action: ActionKind.Accept,
            requestBody: {
                beta_ledger_refund_identity: bob.wallet.eth().address(),
            },
        },
        {
            actor: alice,
            action: ActionKind.Fund,
        },
        {
            actor: bob,
            action: ActionKind.Fund,
        },
        {
            actor: alice,
            action: ActionKind.Redeem,
            afterTest: {
                description:
                    "[alice] Should have received the beta asset after the redeem",
                callback: async function(swapLocations: {
                    [key: string]: string;
                }) {
                    await alice.pollComitNodeUntil(
                        swapLocations["alice"],
                        body => body.state.beta_ledger.status === "Redeemed"
                    );

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
            afterTest: {
                description:
                    "[bob] Should have received the alpha asset after the redeem",
                callback: async function(swapLocations: {
                    [key: string]: string;
                }) {
                    let body = (await bob.pollComitNodeUntil(
                        swapLocations["bob"],
                        body => body.state.alpha_ledger.status === "Redeemed"
                    )) as SwapResponse;
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
