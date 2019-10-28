import { expect, request } from "chai";
import "chai/register-should";
import { ethers } from "ethers";
import { EmbeddedRepresentationSubEntity } from "../gen/siren";
import { Actor } from "../lib/actor";
import "../lib/setup_chai";

(async () => {
    const alpha = {
        ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        asset: {
            name: "bitcoin",
            quantity: {
                bob: "100000000",
                charlie: "200000000",
            },
        },
        expiry: new Date("2080-06-11T23:00:00Z").getTime() / 1000,
    };

    const beta = {
        ledger: {
            name: "ethereum",
            chain_id: 17,
        },
        asset: {
            name: "ether",
            quantity: {
                bob: ethers.utils.parseEther("10").toString(),
                charlie: ethers.utils.parseEther("20").toString(),
            },
        },
        expiry: new Date("2080-06-11T13:00:00Z").getTime() / 1000,
    };

    const alice = new Actor("alice");
    const bob = new Actor("bob");
    const charlie = new Actor("charlie");

    const aliceFinalAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const aliceCndPeerId = await alice.peerId();
    const bobCndPeerId = await bob.peerId();
    const charlieCndPeerId = await charlie.peerId();

    let aliceSwapWithCharlieHref: string;
    let aliceSwapWithBobHref: string;

    describe("SWAP requests to multiple peers", () => {
        it("[Alice] Should be able to send a swap request to Bob", async () => {
            const res = await request(alice.cndHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: alpha.ledger.name,
                        network: alpha.ledger.network,
                    },
                    beta_ledger: {
                        name: beta.ledger.name,
                        chain_id: beta.ledger.chain_id,
                    },
                    alpha_asset: {
                        name: alpha.asset.name,
                        quantity: alpha.asset.quantity.bob,
                    },
                    beta_asset: {
                        name: beta.asset.name,
                        quantity: beta.asset.quantity.bob,
                    },
                    beta_ledger_redeem_identity: aliceFinalAddress,
                    alpha_expiry: alpha.expiry,
                    beta_expiry: beta.expiry,
                    peer: bobCndPeerId,
                });

            res.error.should.equal(false);
            res.should.have.status(201);

            aliceSwapWithBobHref = res.header.location;
            aliceSwapWithBobHref.should.be.a("string");
        });

        it("[Bob] should use the same swap id as Alice", async () => {
            const aliceResponse = await request(alice.cndHttpApiUrl()).get(
                aliceSwapWithBobHref
            );
            const aliceSwapId = aliceResponse.body.properties.id;

            const bobSwap = await bob
                .pollCndUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            expect(bobSwap.properties).to.have.property("id", aliceSwapId);
        });

        it("[Alice] Should be able to send a swap request to Charlie", async () => {
            const res = await request(alice.cndHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: alpha.ledger.name,
                        network: alpha.ledger.network,
                    },
                    beta_ledger: {
                        name: beta.ledger.name,
                        chain_id: beta.ledger.chain_id,
                    },
                    alpha_asset: {
                        name: alpha.asset.name,
                        quantity: alpha.asset.quantity.charlie,
                    },
                    beta_asset: {
                        name: beta.asset.name,
                        quantity: beta.asset.quantity.charlie,
                    },
                    beta_ledger_redeem_identity: aliceFinalAddress,
                    alpha_expiry: alpha.expiry,
                    beta_expiry: beta.expiry,
                    peer: charlieCndPeerId,
                });

            res.error.should.equal(false);
            res.should.have.status(201);

            aliceSwapWithCharlieHref = res.header.location;
        });

        it("[Alice] Should be IN_PROGRESS and SENT after sending the swap request to Charlie", async function() {
            return alice.pollCndUntil(
                aliceSwapWithCharlieHref,
                body =>
                    body.properties.status === "IN_PROGRESS" &&
                    body.properties.state.communication.status === "SENT"
            );
        });

        it("[Alice] Should be able to see Bob's peer-id after sending the swap request to Bob", async function() {
            return alice.pollCndUntil(
                aliceSwapWithBobHref,
                body => body.properties.counterparty === bobCndPeerId
            );
        });

        it("[Alice] Should be able to see Charlie's peer-id after sending the swap request to Charlie", async function() {
            return alice.pollCndUntil(
                aliceSwapWithCharlieHref,
                body => body.properties.counterparty === charlieCndPeerId
            );
        });

        it("[Charlie] Shows the Swap as IN_PROGRESS in /swaps", async () => {
            const swapEntity = await charlie
                .pollCndUntil("/swaps", body => body.entities.length > 0)
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
                .pollCndUntil("/swaps", body => body.entities.length > 0)
                .then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

            expect(swapEntity.properties).to.have.property(
                "counterparty",
                aliceCndPeerId
            );
        });

        it("[Alice] Should see both Bob and Charlie in her list of peers after sending a swap request to both of them", async () => {
            const res = await request(alice.cndHttpApiUrl()).get("/peers");

            res.should.have.status(200);
            res.body.peers.should.have.containSubset([
                {
                    id: bobCndPeerId,
                },
                {
                    id: charlieCndPeerId,
                },
            ]);
        });
    });

    run();
})();
