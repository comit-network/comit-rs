import * as bitcoin from "../../../lib/bitcoin";
import * as chai from "chai";
import * as ethereum from "../../../lib/ethereum";
import { Actor } from "../../../lib/actor";
import { ActionKind, SwapRequest, SwapResponse } from "../../../lib/comit";
import { BN, toBN, toWei } from "web3-utils";
import { HarnessGlobal, sleep } from "../../../lib/util";
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

    const alphaExpiry: number =
        new Date("2080-06-11T13:00:00Z").getTime() / 1000;
    const betaExpiry: number = Math.round(Date.now() / 1000) + 2; // Expires in 2 seconds

    const initialUrl = "/swaps/rfc003";
    const listUrl = "/swaps";

    await bitcoin.ensureSegwit();
    await bob.wallet.eth().fund(bobInitialEth);
    await alice.wallet.eth().fund(aliceInitialEth);
    await alice.wallet.btc().fund(10);
    await bitcoin.generate();

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
            test: {
                description:
                    "[bob] Should have less beta asset after the funding & Waiting for beta htlc to expire",
                callback: async () => {
                    const bobWeiBalanceAfter = await ethereum.ethBalance(
                        bob.wallet.eth().address()
                    );
                    const bobWeiBalanceInit = toWei(toBN(bobInitialEth));

                    bobWeiBalanceAfter
                        .lt(bobWeiBalanceInit)
                        .should.be.equal(true);

                    while (Date.now() / 1000 < betaExpiry + 1) {
                        await sleep(200);
                    }
                },
                timeout: 10000,
            },
        },
        {
            actor: bob,
            action: ActionKind.Refund,
            state: (state: any) => {
                return state.beta_ledger.status === "Refunded";
            },
            test: {
                description:
                    "[bob] Should have received the beta asset after the refund",
                callback: async () => {
                    const bobWeiBalanceAfter = await ethereum.ethBalance(
                        bob.wallet.eth().address()
                    );
                    const bobWeiBalanceInit = toWei(toBN(bobInitialEth));

                    bobWeiBalanceAfter
                        .eq(bobWeiBalanceInit)
                        .should.be.equal(true);
                },
            },
        },
    ];

    describe("RFC003: Bitcoin for Ether - Ether Refunded to Bob", async () => {
        createTests(alice, bob, actions, initialUrl, listUrl, swapRequest);
    });
    run();
})();
