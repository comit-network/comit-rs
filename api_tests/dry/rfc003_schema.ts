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
import { sleep } from "../lib_sdk/utils";
import { EmbeddedRepresentationSubEntity, Entity, Link } from "../gen/siren";
import * as sirenJsonSchema from "../siren.schema.json";
import * as swapPropertiesJsonSchema from "../swap.schema.json";

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
            quantity: alpha.asset.quantity.bob,
        },
        beta_asset: {
            name: beta.asset.name,
            quantity: beta.asset.quantity.bob,
        },
        beta_ledger_redeem_identity:
            "0x00a329c0648769a73afac7f9381e08fb43dbea72",
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

async function assertValidSirenDocument(
    swapsEntity: Entity,
    alice: Actor,
    message: string
) {
    const selfLink = swapsEntity.links.find((link: Link) =>
        link.rel.includes("self")
    ).href;

    const swapResponse = await request(alice.cndHttpApiUrl()).get(selfLink);
    const swapEntity = swapResponse.body as Entity;

    expect(swapEntity, message).to.be.jsonSchema(sirenJsonSchema);
    expect(swapEntity.properties, message).to.be.jsonSchema(
        swapPropertiesJsonSchema
    );
}

setTimeout(async function() {
    describe("Response shape", () => {
        twoActorTest(
            "[Alice] Response for GET /swaps is a valid siren document",
            async function({ alice }) {
                const res = await request(alice.cndHttpApiUrl()).get("/swaps");

                expect(res.body).to.be.jsonSchema(sirenJsonSchema);
            }
        );

        twoActorTest(
            "Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema",
            async function({ alice, bob }) {
                const mockBitcoinWallet = Mock.of<BitcoinWallet>();
                const mockEthereumWallet = Mock.of<EthereumWallet>();

                const aliceComitClient = new ComitClient(
                    mockBitcoinWallet,
                    mockEthereumWallet,
                    alice.cnd
                );

                // Alice send swap request to Bob
                await aliceComitClient.sendSwap(
                    await createDefaultSwapRequest(bob)
                );

                const aliceSwapEntity = await pollCndUntil(
                    alice,
                    "/swaps",
                    body => body.entities.length > 0
                ).then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );
                await assertValidSirenDocument(
                    aliceSwapEntity,
                    alice,
                    "[Alice] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema"
                );

                const bobsSwapEntity = await pollCndUntil(
                    bob,
                    "/swaps",
                    body => body.entities.length > 0
                ).then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );
                await assertValidSirenDocument(
                    bobsSwapEntity,
                    bob,
                    "[Bob] Response for GET /swaps/rfc003/{} is a valid siren document and properties match the json schema"
                );
            }
        );

        twoActorTest(
            "[Alice] Response for GET /swaps/rfc003/{} contains a link to the protocol spec",
            async function({ alice, bob }) {
                const mockBitcoinWallet = Mock.of<BitcoinWallet>();
                const mockEthereumWallet = Mock.of<EthereumWallet>();

                const aliceComitClient = new ComitClient(
                    mockBitcoinWallet,
                    mockEthereumWallet,
                    alice.cnd
                );

                // Alice send swap request to Bob
                await aliceComitClient.sendSwap(
                    await createDefaultSwapRequest(bob)
                );

                const aliceSwapEntity = await pollCndUntil(
                    alice,
                    "/swaps",
                    body => body.entities.length > 0
                ).then(
                    body => body.entities[0] as EmbeddedRepresentationSubEntity
                );

                const protocolLink = aliceSwapEntity.links.find((link: Link) =>
                    link.rel.includes("describedBy")
                );

                expect(protocolLink).to.be.deep.equal({
                    rel: ["describedBy"],
                    class: ["protocol-spec"],
                    type: "text/html",
                    href:
                        "https://github.com/comit-network/RFCs/blob/master/RFC-003-SWAP-Basic.adoc",
                });
            }
        );
    });

    run();
}, 0);
