// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
// These test do not use the sdk so that we can test edge cases
import { twoActorTest } from "../lib_sdk/actor_test";
import { expect, request } from "chai";
import "chai/register-should";
import { ethers } from "ethers";
import "../lib/setup_chai";
import { Actor } from "../lib_sdk/actors/actor";
import {
    BitcoinWallet,
    ComitClient,
    EthereumWallet,
    SwapRequest,
} from "comit-sdk";
import { Mock } from "ts-mockery";
import { EmbeddedRepresentationSubEntity, Entity } from "../gen/siren";
import { sleep } from "../lib_sdk/utils";
import * as swapPropertiesJsonSchema from "../swap.schema.json";

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
        chain_id: 17,
    },
    asset: {
        name: "ether",
        quantity: ethers.utils.parseEther("10").toString(),
    },
    expiry: new Date("2080-06-11T13:00:00Z").getTime() / 1000,
};
const aliceFinalAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

async function createDefaultSwapRequest(counterParty: Actor) {
    const swapRequest: SwapRequest = {
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
            quantity: alpha.asset.quantity.reasonable,
        },
        beta_asset: {
            name: beta.asset.name,
            quantity: beta.asset.quantity,
        },
        beta_ledger_redeem_identity: aliceFinalAddress,
        alpha_expiry: alpha.expiry,
        beta_expiry: beta.expiry,
        peer: {
            peer_id: await counterParty.cnd.getPeerId(),
            address_hint: await counterParty.cnd
                .getPeerListenAddresses()
                .then(addresses => addresses[0]),
        },
    };
    return swapRequest;
}

async function assertSwapsInProgress(actor: Actor, message: string) {
    const res = await request(actor.cndHttpApiUrl()).get("/swaps");

    const swapEntities = res.body.entities as EmbeddedRepresentationSubEntity[];

    expect(swapEntities.map(entity => entity.properties, message))
        .to.each.have.property("status")
        .that.is.equal("IN_PROGRESS");
}

async function pollCndUntil(
    actor: Actor,
    location: string,
    predicate: (body: Entity) => boolean
): Promise<Entity> {
    const response = await request(actor.cndHttpApiUrl()).get(location);

    expect(response).to.have.status(200);

    if (predicate(response.body)) {
        return response.body;
    } else {
        await sleep(500);

        return this.pollCndUntil(location, predicate);
    }
}

setTimeout(async function() {
    describe("SWAP request DECLINED", () => {
        twoActorTest(
            "[Alice] Should be able to make first swap request via HTTP api",
            async function({ alice, bob }) {
                // setup

                const mockBitcoinWallet = Mock.of<BitcoinWallet>();
                const mockEthereumWallet = Mock.of<EthereumWallet>();

                const aliceComitClient = new ComitClient(
                    mockBitcoinWallet,
                    mockEthereumWallet,
                    alice.cnd
                );

                // Alice should be able to send two swap requests to Bob
                await aliceComitClient.sendSwap({
                    ...(await createDefaultSwapRequest(bob)),
                    alpha_asset: {
                        name: alpha.asset.name,
                        quantity: alpha.asset.quantity.reasonable,
                    },
                });
                await aliceComitClient.sendSwap({
                    ...(await createDefaultSwapRequest(bob)),
                    alpha_asset: {
                        name: alpha.asset.name,
                        quantity: alpha.asset.quantity.stingy,
                    },
                });

                await assertSwapsInProgress(
                    alice,
                    "[Alice] Shows the swaps as IN_PROGRESS in GET /swaps"
                );
                await assertSwapsInProgress(
                    bob,
                    "[Bob] Shows the swaps as IN_PROGRESS in /swaps"
                );
            }
        );

        twoActorTest("[Bob] Decline one swap", async function({ alice, bob }) {
            // setup

            const mockBitcoinWallet = Mock.of<BitcoinWallet>();
            const mockEthereumWallet = Mock.of<EthereumWallet>();

            const aliceComitClient = new ComitClient(
                mockBitcoinWallet,
                mockEthereumWallet,
                alice.cnd
            );

            // Alice should be able to send two swap requests to Bob
            const aliceReasonableSwap = await aliceComitClient.sendSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: alpha.asset.name,
                    quantity: alpha.asset.quantity.reasonable,
                },
            });
            const aliceStingySwap = await aliceComitClient.sendSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: alpha.asset.name,
                    quantity: alpha.asset.quantity.stingy,
                },
            });

            const swapEntities = await pollCndUntil(
                bob,
                "/swaps",
                body => body.entities.length === 2
            ).then(body => body.entities as EmbeddedRepresentationSubEntity[]);

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

            const bobStingySwapHref = stingySwap.links.find(link =>
                link.rel.includes("self")
            ).href;

            const res = await request(bob.cndHttpApiUrl()).get(
                bobStingySwapHref
            );

            const body = res.body as Entity;
            expect(
                body.properties,
                "[Bob] Has the RFC-003 parameters when GETing the swap"
            ).jsonSchema(swapPropertiesJsonSchema);
            expect(
                body.actions,
                "[Bob] Has the accept and decline actions when GETing the swap"
            ).containSubset([
                {
                    name: "accept",
                },
                {
                    name: "decline",
                },
            ]);

            /// Decline the swap
            const decline = body.actions.find(
                action => action.name === "decline"
            );
            const declineRes = await bob.cnd.executeAction(decline);

            declineRes.should.have.status(200);
            expect(
                await pollCndUntil(
                    bob,
                    bobStingySwapHref,
                    entity =>
                        entity.properties.state.communication.status ===
                        "DECLINED"
                ),
                "[Bob] Should be in the Declined State after declining a swap request providing a reason"
            ).to.exist;

            const aliceStingySwapDetails = await aliceStingySwap.fetchDetails();
            expect(
                aliceStingySwapDetails.properties.state.communication.status,
                "[Alice] Should be in the Declined State after Bob declines a swap"
            ).to.eq("DECLINED");

            const aliceReasonableSwapDetails = await aliceReasonableSwap.fetchDetails();
            expect(
                aliceReasonableSwapDetails.properties.state.communication
                    .status,
                "[Alice] Should be in the SENT State for the other swap request"
            ).to.eq("SENT");
        });
    });

    run();
}, 0);
