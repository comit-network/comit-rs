import { expect, request } from "chai";
import "chai/register-should";
import { toWei } from "web3-utils";
import { EmbeddedRepresentationSubEntity, Entity } from "../gen/siren";
import { Actor } from "../lib/actor";
import "../lib/setup_chai";
import { HarnessGlobal, sleep } from "../lib/util";
import * as swapPropertiesJsonSchema from "../swap.schema.json";

declare var global: HarnessGlobal;

(async function() {
    const alpha = {
        ledger: {
            name: "bitcoin",
            network: "regtest",
        },
        asset: {
            name: "bitcoin",
            quantity: {
                reasonable: "100000000",
                stingy: "100",
            },
        },
        expiry: new Date("2080-06-11T23:00:00Z").getTime() / 1000,
    };

    const beta = {
        ledger: {
            name: "ethereum",
            network: "regtest",
        },
        asset: {
            name: "ether",
            quantity: toWei("10", "ether"),
        },

        expiry: new Date("2080-06-11T13:00:00Z").getTime() / 1000,
    };

    const alice = new Actor("alice", global.config, global.project_root);
    const bob = new Actor("bob", global.config, global.project_root);
    const aliceFinalAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    const bobCndPeerId = await bob.peerId();

    describe("SWAP request REJECTED", () => {
        let aliceReasonableSwapHref: string;
        it("[Alice] Should be able to make first swap request via HTTP api", async () => {
            const res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: alpha.ledger.name,
                        network: alpha.ledger.network,
                    },
                    beta_ledger: {
                        name: beta.ledger.name,
                        network: beta.ledger.network,
                    },
                    alpha_asset: {
                        name: alpha.asset.name,
                        quantity: alpha.asset.quantity.reasonable,
                    },
                    beta_asset: {
                        name: beta.asset.name,
                        quantity: beta.asset.quantity,
                    },
                    beta_ledger_redeem_identity: aliceFinalAddress,
                    alpha_expiry: alpha.expiry,
                    beta_expiry: beta.expiry,
                    peer: bobCndPeerId,
                });

            res.error.should.equal(false);
            res.should.have.status(201);
            const swapLocation = res.header.location;
            swapLocation.should.be.a("string");
            aliceReasonableSwapHref = swapLocation;
        });

        it("[Alice] Should see Bob in her list of peers after sending a swap request to him", async () => {
            await sleep(1000);
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/peers"
            );

            res.should.have.status(200);
            res.body.peers.should.containSubset([
                {
                    id: bobCndPeerId,
                },
            ]);
        });

        it("[Bob] Should see a new peer in his list of peers after receiving a swap request from Alice", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get("/peers");

            res.should.have.status(200);
            res.body.peers.should.have.length(1);
        });

        let aliceStingySwapHref: string;
        it("[Alice] Should be able to make second swap request via HTTP api", async () => {
            const res = await request(alice.comitNodeHttpApiUrl())
                .post("/swaps/rfc003")
                .send({
                    alpha_ledger: {
                        name: alpha.ledger.name,
                        network: alpha.ledger.network,
                    },
                    beta_ledger: {
                        name: beta.ledger.name,
                        network: beta.ledger.network,
                    },
                    alpha_asset: {
                        name: alpha.asset.name,
                        quantity: alpha.asset.quantity.stingy,
                    },
                    beta_asset: {
                        name: beta.asset.name,
                        quantity: beta.asset.quantity,
                    },
                    beta_ledger_redeem_identity: aliceFinalAddress,
                    alpha_expiry: alpha.expiry,
                    beta_expiry: beta.expiry,
                    peer: bobCndPeerId,
                });

            res.error.should.equal(false);
            res.should.have.status(201);

            const swapLocation = res.header.location;
            swapLocation.should.be.a("string");
            aliceStingySwapHref = swapLocation;
        });

        it("[Alice] Should still only see Bob in her list of peers after sending a second swap request to him", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/peers"
            );

            res.should.have.status(200);
            res.body.peers.should.containSubset([
                {
                    id: bobCndPeerId,
                },
            ]);
        });

        it("[Bob] Should still only see one peer in his list of peers after receiving a second swap request from Alice", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get("/peers");

            res.should.have.status(200);
            res.body.peers.should.have.length(1);
        });

        it("[Alice] Shows the swaps as IN_PROGRESS in GET /swaps", async () => {
            const res = await request(alice.comitNodeHttpApiUrl()).get(
                "/swaps"
            );

            res.should.have.status(200);

            const swapEntities = res.body
                .entities as EmbeddedRepresentationSubEntity[];

            expect(swapEntities.map(entity => entity.properties))
                .to.each.have.property("status")
                .that.is.equal("IN_PROGRESS");
        });

        let bobStingySwapHref: string;
        let bobReasonableSwapHref: string;

        it("[Bob] Shows the swaps as IN_PROGRESS in /swaps", async () => {
            const swapEntities = await bob
                .pollComitNodeUntil(
                    "/swaps",
                    body => body.entities.length === 2
                )
                .then(
                    body => body.entities as EmbeddedRepresentationSubEntity[]
                );

            expect(swapEntities.map(entity => entity.properties))
                .to.each.have.property("protocol")
                .that.is.equal("rfc003");
            expect(swapEntities.map(entity => entity.properties))
                .to.each.have.property("status")
                .that.is.equal("IN_PROGRESS");

            const stingySwap = swapEntities.find(entity => {
                return (
                    parseInt(
                        entity.properties.parameters.alpha_asset.quantity,
                        10
                    ) === parseInt(alpha.asset.quantity.stingy, 10)
                );
            });
            const reasonableSwap = swapEntities.find(entity => {
                return (
                    parseInt(
                        entity.properties.parameters.alpha_asset.quantity,
                        10
                    ) === parseInt(alpha.asset.quantity.reasonable, 10)
                );
            });

            bobStingySwapHref = stingySwap.links.find(link =>
                link.rel.includes("self")
            ).href;
            bobReasonableSwapHref = reasonableSwap.links.find(link =>
                link.rel.includes("self")
            ).href;
        });

        it("[Bob] Has the RFC-003 parameters when GETing the swap", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get(
                bobStingySwapHref
            );

            res.should.have.status(200);

            const body = res.body as Entity;

            expect(body.properties).jsonSchema(swapPropertiesJsonSchema);
        });

        it("[Bob] Has the accept and decline actions when GETing the swap", async () => {
            const res = await request(bob.comitNodeHttpApiUrl()).get(
                bobStingySwapHref
            );

            res.should.have.status(200);

            const body = res.body as Entity;

            expect(body.actions).containSubset([
                {
                    name: "accept",
                },
                {
                    name: "decline",
                },
            ]);
        });

        it("[Bob] Can execute a decline action", async () => {
            const bob = new Actor(
                "bob",
                global.config,
                global.project_root,
                null,
                {
                    reason: "BadRate",
                }
            );

            const res = await request(bob.comitNodeHttpApiUrl()).get(
                bobStingySwapHref
            );
            const body = res.body as Entity;

            const decline = body.actions.find(
                action => action.name === "decline"
            );
            const declineRes = await bob.doComitAction(decline);

            declineRes.should.have.status(200);
        });

        it("[Bob] Should be in the Rejected State after declining a swap request providing a reason", async function() {
            await bob.pollComitNodeUntil(
                bobStingySwapHref,
                entity =>
                    entity.properties.state.communication.status === "REJECTED"
            );
        });

        it("[Alice] Should be in the Rejected State after Bob declines a swap request providing a reason", async () => {
            await alice.pollComitNodeUntil(
                aliceStingySwapHref,
                body =>
                    body.properties.state.communication.status === "REJECTED"
            );
        });

        it("[Bob] Can execute a decline action, without providing a reason", async () => {
            const bob = new Actor("bob", global.config, global.project_root);

            const res = await request(bob.comitNodeHttpApiUrl()).get(
                bobReasonableSwapHref
            );
            const body = res.body as Entity;

            const decline = body.actions.find(
                action => action.name === "decline"
            );
            const declineRes = await bob.doComitAction(decline);

            declineRes.should.have.status(200);
        });

        it("[Bob] Should be in the Rejected State after declining a swap request without a reason", async () => {
            await bob.pollComitNodeUntil(
                bobReasonableSwapHref,
                entity =>
                    entity.properties.state.communication.status === "REJECTED"
            );
        });

        it("[Alice] Should be in the Rejected State after Bob declines a swap request without a reason", async () => {
            await alice.pollComitNodeUntil(
                aliceReasonableSwapHref,
                entity =>
                    entity.properties.state.communication.status === "REJECTED"
            );
        });
    });

    run();
})();
