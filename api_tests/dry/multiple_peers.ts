import { expect, request } from "chai";
import "chai/register-should";
import { toWei } from "web3-utils";
import { EmbeddedRepresentationSubEntity } from "../gen/siren";
import { Actor } from "../lib/actor";
import "../lib/setup_chai";
import { HarnessGlobal } from "../lib/util";

declare var global: HarnessGlobal;

(async () => {
    const alphaLedgerName = "bitcoin";
    const alphaLedgerNetwork = "regtest";

    const betaLedgerName = "ethereum";
    const betaLedgerNetwork = "regtest";

    const alphaAssetName = "bitcoin";
    const alphaAssetBobQuantity = "100000000";
    const alphaAssetCharlieQuantity = "200000000";

    const betaAssetName = "ether";
    const betaAssetBobQuantity = toWei("10", "ether");
    const betaAssetCharlieQuantity = toWei("20", "ether");

    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    const alice = new Actor("alice", global.config, global.project_root);
    const bob = new Actor("bob", global.config, global.project_root);
    const charlie = new Actor("charlie", global.config, global.project_root);

    const aliceFinalAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const aliceCndAddress = await alice.peerId();
    const bobCndAddress = await bob.peerId();
    const charlieCndAddress = await charlie.peerId();

    let aliceSwapWithCharlieHref: string;
    let aliceSwapWithBobHref: string;

    describe("SWAP requests to multiple peers", () => {
        it("[Alice] Should be able to send a swap request to Bob", async () => {
            const res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: alphaLedgerName,
                        network: alphaLedgerNetwork,
                    },
                    beta_ledger: {
                        name: betaLedgerName,
                        network: betaLedgerNetwork,
                    },
                    alpha_asset: {
                        name: alphaAssetName,
                        quantity: alphaAssetBobQuantity,
                    },
                    beta_asset: {
                        name: betaAssetName,
                        quantity: betaAssetBobQuantity,
                    },
                    beta_ledger_redeem_identity: aliceFinalAddress,
                    alpha_expiry: alphaExpiry,
                    beta_expiry: betaExpiry,
                    peer: bobCndAddress,
                });

            res.error.should.equal(false);
            res.should.have.status(201);

            aliceSwapWithBobHref = res.header.location;
            aliceSwapWithBobHref.should.be.a("string");
        });

        it("[Alice] Should be able to send a swap request to Charlie", async () => {
            const res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: alphaLedgerName,
                        network: alphaLedgerNetwork,
                    },
                    beta_ledger: {
                        name: betaLedgerName,
                        network: betaLedgerNetwork,
                    },
                    alpha_asset: {
                        name: alphaAssetName,
                        quantity: alphaAssetCharlieQuantity,
                    },
                    beta_asset: {
                        name: betaAssetName,
                        quantity: betaAssetCharlieQuantity,
                    },
                    beta_ledger_redeem_identity: aliceFinalAddress,
                    alpha_expiry: alphaExpiry,
                    beta_expiry: betaExpiry,
                    peer: charlieCndAddress,
                });

            res.error.should.equal(false);
            res.should.have.status(201);

            aliceSwapWithCharlieHref = res.header.location;
        });

        it("[Alice] Should be IN_PROGRESS and SENT after sending the swap request to Charlie", async function() {
            return alice.pollComitNodeUntil(
                aliceSwapWithCharlieHref,
                body =>
                    body.properties.status === "IN_PROGRESS" &&
                    body.properties.state.communication.status === "SENT"
            );
        });

        it("[Alice] Should be able to see Bob's peer-id after sending the swap request to Bob", async function() {
            return alice.pollComitNodeUntil(
                aliceSwapWithBobHref,
                body => body.properties.counterparty === bobCndAddress
            );
        });

        it("[Alice] Should be able to see Charlie's peer-id after sending the swap request to Charlie", async function() {
            return alice.pollComitNodeUntil(
                aliceSwapWithCharlieHref,
                body => body.properties.counterparty === charlieCndAddress
            );
        });

        it("[Charlie] Shows the Swap as IN_PROGRESS in /swaps", async () => {
            const swapEntity = await charlie
                .pollComitNodeUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            expect(swapEntity.properties).to.have.property(
                "protocol",
                "rfc003"
            );
            expect(swapEntity.properties).to.have.property(
                "status",
                "IN_PROGRESS"
            );
            expect(swapEntity.links).containSubset([
                {
                    rel: (expectedValue: string[]) =>
                        expectedValue.includes("self"),
                },
            ]);
        });

        it("[Charlie] Should be able to see Alice's peer-id after receiving the request", async function() {
            const swapEntity = await charlie
                .pollComitNodeUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            expect(swapEntity.properties).to.have.property(
                "counterparty",
                aliceCndAddress
            );
        });

        it("[Alice] Should see both Bob and Charlie in her list of peers after sending a swap request to both of them", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/peers"
            );

            res.should.have.status(200);
            res.body.peers.should.have.containSubset([
                {
                    id: bobCndAddress,
                },
                {
                    id: charlieCndAddress,
                },
            ]);
        });
    });

    run();
})();
