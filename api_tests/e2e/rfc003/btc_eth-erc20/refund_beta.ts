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
    });

    const alphaAssetQuantity = 100000000;
    const betaAssetQuantity = ethers.utils.parseEther("5000");
    const bobInitialErc20 = ethers.utils.parseEther("10000");

    const alphaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;
    const betaExpiry = Math.round(Date.now() / 1000) + 9;

    await bitcoin.ensureFunding();
    await bob.wallet.eth().fund("5");

    await alice.wallet.btc().fund(10);
    await bitcoin.generate();
    await alice.wallet.eth().fund("1");

    const tokenContractAddress = await tobyWallet
        .eth()
        .deployErc20TokenContract(global.projectRoot);
    await tobyWallet.eth().fund("10");
    await tobyWallet
        .eth()
        .mintErc20To(
            bob.wallet.eth().address(),
            bobInitialErc20,
            tokenContractAddress
        );

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
            name: "erc20",
            quantity: betaAssetQuantity.toString(),
            token_contract: tokenContractAddress,
        },
        beta_ledger_redeem_identity: alice.wallet.eth().address(),
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
        peer: await bob.peerId(),
    };

    const erc20Balance = await bob.wallet
        .eth()
        .erc20Balance(tokenContractAddress);
    erc20Balance.eq(bobInitialErc20).should.equal(true);

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
            action: ActionKind.Deploy,
            waitUntil: state => state.beta_ledger.status === "DEPLOYED",
        },
        {
            actor: bob,
            action: ActionKind.Fund,
            waitUntil: state => state.beta_ledger.status === "FUNDED",
            test: {
                description: "Should have less beta asset after the funding",
                callback: async () => {
                    const bobErc20BalanceAfter = await bob.wallet
                        .eth()
                        .erc20Balance(tokenContractAddress);

                    bobErc20BalanceAfter
                        .lt(bobInitialErc20)
                        .should.be.equal(true);
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
                    const bobErc20BalanceAfter = await bob.wallet
                        .eth()
                        .erc20Balance(tokenContractAddress);

                    bobErc20BalanceAfter
                        .eq(bobInitialErc20)
                        .should.be.equal(true);
                },
            },
        },
    ];

    describe("RFC003: Bitcoin for ERC20 - ERC20 (beta) refunded to Bob", async () => {
        createTests(alice, bob, steps, "/swaps/rfc003", "/swaps", swapRequest);
    });
    run();
})();
