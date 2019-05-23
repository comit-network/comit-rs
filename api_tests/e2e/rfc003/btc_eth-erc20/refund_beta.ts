import * as bitcoin from "../../../lib/bitcoin";
import { Actor } from "../../../lib/actor";
import { ActionKind, SwapRequest } from "../../../lib/comit";
import { Wallet } from "../../../lib/wallet";
import { toBN, toWei } from "web3-utils";
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
    });
    const bob = new Actor("bob", global.config, global.project_root, {
        ethereumNodeConfig: global.ledgers_config.ethereum,
        bitcoinNodeConfig: global.ledgers_config.bitcoin,
    });

    const alphaAssetQuantity = 100000000;
    const betaAssetQuantity = toBN(toWei("5000", "ether"));
    const bobInitialErc20 = toBN(toWei("10000", "ether"));

    const alphaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;
    const betaExpiry = Math.round(Date.now() / 1000) + 9;

    await bitcoin.ensureFunding();
    await bob.wallet.eth().fund("5");

    await alice.wallet.btc().fund(10);
    await bitcoin.generate();
    await alice.wallet.eth().fund("1");

    let tokenContractAddress = await tobyWallet
        .eth()
        .deployErc20TokenContract(global.project_root);
    await tobyWallet.eth().fund("10");
    await tobyWallet
        .eth()
        .mintErc20To(
            bob.wallet.eth().address(),
            bobInitialErc20,
            tokenContractAddress
        );

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
        beta_ledger_redeem_identity: alice.wallet.eth().address(),
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
        peer: await bob.peerId(),
    };

    let erc20Balance = await bob.wallet
        .eth()
        .erc20Balance(tokenContractAddress);
    erc20Balance.eq(bobInitialErc20).should.equal(true);

    const actions: ActionTrigger[] = [
        {
            actor: bob,
            action: ActionKind.Accept,
            state: state => state.communication.status === "ACCEPTED",
        },
        {
            actor: alice,
            action: ActionKind.Fund,
            state: state => state.alpha_ledger.status === "Funded",
        },
        {
            actor: bob,
            action: ActionKind.Deploy,
            state: state => state.beta_ledger.status === "Deployed",
        },
        {
            actor: bob,
            action: ActionKind.Fund,
            state: state => state.beta_ledger.status === "Funded",
            test: {
                description: "Should have less beta asset after the funding",
                callback: async () => {
                    let bobErc20BalanceAfter = await bob.wallet
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
            state: state => state.beta_ledger.status === "Refunded",
            test: {
                description:
                    "Should have received the beta asset after the refund",
                callback: async () => {
                    let bobErc20BalanceAfter = await bob.wallet
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
