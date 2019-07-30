import BN = require("bn.js");
import "chai/register-should";
import { toBN, toWei } from "web3-utils";
import { Actor } from "../../../lib/actor";
import * as bitcoin from "../../../lib/bitcoin";
import { ActionKind, LedgerAction, SwapRequest } from "../../../lib/comit";
import "../../../lib/setup_chai";
import {
    createTests,
    hasAction,
    mapToAction,
    Step,
} from "../../../lib/test_creator";
import { HarnessGlobal } from "../../../lib/util";

declare var global: HarnessGlobal;

(async function() {
    const alice = new Actor("alice", global.config, global.project_root, {
        ethereumNodeConfig: global.ledgers_config.ethereum,
        bitcoinNodeConfig: global.ledgers_config.bitcoin,
    });
    const bob = new Actor("bob", global.config, global.project_root, {
        ethereumNodeConfig: global.ledgers_config.ethereum,
        bitcoinNodeConfig: global.ledgers_config.bitcoin,
    });

    const alphaAssetQuantity = 100000000;
    const betaAssetQuantity = toBN(toWei("10", "ether"));

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    await bitcoin.ensureFunding();
    await bob.wallet.eth().fund("11");
    await alice.wallet.eth().fund("0.1");
    await alice.wallet.btc().fund(20);
    await bitcoin.generate();

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
            name: "ether",
            quantity: betaAssetQuantity.toString(),
        },
        beta_ledger_redeem_identity: alice.wallet.eth().address(),
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
        // given an over-funded HTLC
        {
            actor: alice,
            action: {
                description: "can overfund the bitcoin HTLC",
                exec: async (actor, swapHref) => {
                    const sirenAction = await actor
                        .pollCndUntil(swapHref, hasAction(ActionKind.Fund))
                        .then(mapToAction(ActionKind.Fund));

                    const response = await actor.doComitAction(sirenAction);
                    const ledgerAction = response.body as LedgerAction;

                    if (
                        !(
                            "bitcoin-send-amount-to-address" ===
                            ledgerAction.type
                        )
                    ) {
                        throw new Error(
                            `Expected ledger action to be 'bitcoin-send-amount-to-address' but was '${ledgerAction.type}'`
                        );
                    }

                    // @ts-ignore
                    ledgerAction.payload.amount = new BN(
                        ledgerAction.payload.amount,
                        10
                    )
                        .mul(new BN(10))
                        .toString(10);

                    await actor.doLedgerAction(ledgerAction);
                },
            },
        },
        // alice should not consider the HTLC to be funded and terminate with NOT_SWAPPED
        {
            actor: alice,
            waitUntil: state => state.alpha_ledger.status === "InvalidFunded", // alpha ledger invalid funding and check for single refund action
        },
        // bob should not consider the HTLC to be funded and terminate with NOT_SWAPPED
        {
            actor: bob,
            waitUntil: state => state.status === "NOT_SWAPPED",
        },
    ];

    describe("RFC003: Bitcoin for Ether - overfunded HTLC", async () => {
        createTests(alice, bob, steps, "/swaps/rfc003", "/swaps", swapRequest);
    });
    run();
})();
