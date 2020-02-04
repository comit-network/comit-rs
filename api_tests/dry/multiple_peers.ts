// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
// These test do not use the sdk so that we can test edge cases
import { threeActorTest } from "../lib_sdk/actor_test";
import { expect } from "chai";
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
import { SwapDetails } from "comit-sdk/dist/src/cnd";

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
            quantity: alpha.asset.quantity.bob,
        },
        beta_asset: {
            name: beta.asset.name,
            quantity: beta.asset.quantity.bob,
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

async function fetchSwapDetails(
    comitClient: ComitClient
): Promise<SwapDetails[]> {
    let swaps = await comitClient.getNewSwaps();
    while (swaps.length < 1) {
        swaps = await comitClient.getNewSwaps();
        await sleep(1000);
    }
    return Promise.all(swaps.map(swap => swap.fetchDetails()));
}

interface MatchInterface {
    id: string;
    status: string;
    state: string;
}

function toMatch(swapDetail: SwapDetails): MatchInterface {
    return {
        id: swapDetail.properties.id,
        status: swapDetail.properties.status,
        state: swapDetail.properties.state.communication.status,
    };
}

setTimeout(async function() {
    describe("SWAP requests to multiple peers", () => {
        threeActorTest(
            "[Alice] Should be able to send a swap request to Bob and Charlie",
            async function({ alice, bob, charlie }) {
                // setup

                const mockBitcoinWallet = Mock.of<BitcoinWallet>();
                const mockEthereumWallet = Mock.of<EthereumWallet>();

                const aliceComitClient = new ComitClient(
                    mockBitcoinWallet,
                    mockEthereumWallet,
                    alice.cnd
                );
                const bobComitClient = new ComitClient(
                    mockBitcoinWallet,
                    mockEthereumWallet,
                    bob.cnd
                );
                const charlieComitClient = new ComitClient(
                    mockBitcoinWallet,
                    mockEthereumWallet,
                    charlie.cnd
                );

                // Alice send swap request to Bob
                const aliceToBobSwap = await aliceComitClient.sendSwap(
                    await createDefaultSwapRequest(bob)
                );

                // Alice send swap request to Charlie
                const aliceToCharlieSwap = await aliceComitClient.sendSwap(
                    await createDefaultSwapRequest(charlie)
                );

                // fetch swap details
                const aliceToBobSwapDetails = await aliceToBobSwap.fetchDetails();
                const aliceToCharlieSwapDetails = await aliceToCharlieSwap.fetchDetails();

                // Bob get swap details
                const bobSwapDetails = (
                    await fetchSwapDetails(bobComitClient)
                )[0];

                // Charlie get swap details
                const charlieSwapDetails = (
                    await fetchSwapDetails(charlieComitClient)
                )[0];

                // retrieve swap from alice for bob
                const aliceBobSwap = await aliceComitClient.retrieveSwapById(
                    bobSwapDetails.properties.id
                );

                // retrieve swap from alice for charlie
                const aliceCharlieSwap = await aliceComitClient.retrieveSwapById(
                    charlieSwapDetails.properties.id
                );

                expect(
                    bobSwapDetails.properties,
                    "[Bob] should have same id as Alice"
                ).to.have.property("id", aliceToBobSwapDetails.properties.id);
                expect(
                    charlieSwapDetails.properties,
                    "[Charlie] should have same id as Alice"
                ).to.have.property(
                    "id",
                    aliceToCharlieSwapDetails.properties.id
                );

                expect(
                    [
                        await aliceBobSwap.fetchDetails(),
                        await aliceCharlieSwap.fetchDetails(),
                    ].map(swapDetail => toMatch(swapDetail))
                ).to.have.deep.members([
                    toMatch(bobSwapDetails),
                    toMatch(charlieSwapDetails),
                ]);
            }
        );
    });

    run();
}, 0);
