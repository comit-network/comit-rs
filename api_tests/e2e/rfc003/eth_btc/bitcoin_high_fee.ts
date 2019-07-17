import { expect, request } from "chai";
import "chai/register-should";
import { toBN, toWei } from "web3-utils";
import { Actor } from "../../../lib/actor";
import * as bitcoin from "../../../lib/bitcoin";
import { ActionKind, SwapRequest } from "../../../lib/comit";
import "../../../lib/setupChai";
import { createTests, Step } from "../../../lib/test_creator";
import { HarnessGlobal } from "../../../lib/util";

declare var global: HarnessGlobal;

(async function() {
    const alice = new Actor("alice", global.config, global.project_root, {
        ethereumNodeConfig: global.ledgers_config.ethereum,
        bitcoinNodeConfig: global.ledgers_config.bitcoin,
        addressForIncomingBitcoinPayments:
            "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0",
    });
    const bob = new Actor("bob", global.config, global.project_root, {
        ethereumNodeConfig: global.ledgers_config.ethereum,
        bitcoinNodeConfig: global.ledgers_config.bitcoin,
        addressForIncomingBitcoinPayments:
            "bcrt1qs2aderg3whgu0m8uadn6dwxjf7j3wx97kk2qqtrum89pmfcxknhsf89pj0",
    });

    const alphaAssetQuantity = toBN(toWei("10", "ether"));
    const betaAssetQuantity = 100000000;

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    await bitcoin.ensureFunding();
    await alice.wallet.eth().fund("11");
    await alice.wallet.btc().fund(0.1);
    await bob.wallet.eth().fund("0.1");
    await bob.wallet.btc().fund(10);
    await bitcoin.generate();

    const swapRequest: SwapRequest = {
        alpha_ledger: {
            name: "ethereum",
            network: "regtest",
        },
        beta_ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        alpha_asset: {
            name: "ether",
            quantity: alphaAssetQuantity.toString(),
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

    const steps: Step[] = [
        {
            actor: bob,
            action: ActionKind.Accept,
            waitUntil: state => state.communication.status === "ACCEPTED",
        },
        {
            actor: alice,
            action: ActionKind.Fund,
            waitUntil: state => state.alpha_ledger.status === "Funded",
        },
        {
            actor: bob,
            action: ActionKind.Fund,
            waitUntil: state => state.beta_ledger.status === "Funded",
        },
    ];

    describe("RFC003: Ether for Bitcoin", () => {
        const swapLocations = createTests(
            alice,
            bob,
            steps,
            "/swaps/rfc003",
            "/swaps",
            swapRequest
        );

        it("[alice] should return a High Fee Error when getting redeem payload with a high fee", async () => {
            const action = await alice
                .pollComitNodeUntil(
                    swapLocations[alice.name],
                    body =>
                        body.actions.findIndex(
                            candidate => candidate.name === ActionKind.Redeem
                        ) !== -1
                )
                .then(body =>
                    body.actions.find(
                        candidate => candidate.name === ActionKind.Redeem
                    )
                );

            const { url, body, method } = alice.buildRequestFromAction(action, {
                bitcoinFeePerWU: 100000000,
            });

            const agent = request(alice.comitNodeHttpApiUrl());

            expect(method).to.equal("GET");

            const response = await agent.get(url).send(body);

            expect(response).to.have.status(400);

            expect(response.body.title).to.equal("Fee is too high.");
        });

        it("[bob] should return a High Fee Error when getting refund payload with a high fee", async () => {
            const action = await bob
                .pollComitNodeUntil(
                    swapLocations[bob.name],
                    body =>
                        body.actions.findIndex(
                            candidate => candidate.name === ActionKind.Refund
                        ) !== -1
                )
                .then(body =>
                    body.actions.find(
                        candidate => candidate.name === ActionKind.Refund
                    )
                );

            const { url, body, method } = bob.buildRequestFromAction(action, {
                bitcoinFeePerWU: 100000000,
            });

            const agent = request(bob.comitNodeHttpApiUrl());

            expect(method).to.equal("GET");

            const response = await agent.get(url).send(body);

            expect(response).to.have.status(400);

            expect(response.body.title).to.equal("Fee is too high.");
        });
    });
    run();
})();
