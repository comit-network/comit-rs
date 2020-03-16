/**
 * @logDir multiple_peers
 */

import { threeActorTest } from "../../lib/actor_test";
import { expect } from "chai";
import { SwapDetails, SwapRequest } from "comit-sdk";
import { Actor } from "../../lib/actors/actor";

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

async function createSwapRequest(counterParty: Actor) {
    const swapRequest: SwapRequest = {
        alpha_ledger: {
            name: "bitcoin",
            network: "testnet",
        },
        alpha_asset: {
            name: "bitcoin",
            quantity: "1000",
        },
        beta_ledger: {
            name: "ethereum",
            chain_id: 3,
        },
        beta_asset: {
            name: "ether",
            quantity: "100",
        },
        alpha_expiry: 1000,
        beta_expiry: 1000,
        beta_ledger_redeem_identity:
            "0xcb46c0e906950CaD69906815e21a84794D5da07a",
        peer: {
            peer_id: await counterParty.cnd.getPeerId(),
            address_hint: await counterParty.cnd
                .getPeerListenAddresses()
                .then(addresses => addresses[0]),
        },
    };
    return swapRequest;
}

// ******************************************** //
// Multiple peers                               //
// ******************************************** //
describe("Reproduce #2214", () => {
    it("reproduce-2214", async function() {
        await threeActorTest(async function({ alice, bob }) {
            // Alice send swap request to Bob
            const aliceToBobSwapUrl = await alice.cnd.postSwap(
                await createSwapRequest(bob)
            );

            // fetch swap details
            const aliceToBobSwapDetails = await alice.pollSwapDetails(
                aliceToBobSwapUrl
            );

            // Bob get swap details
            const bobSwapDetails = await bob.pollSwapDetails(aliceToBobSwapUrl);

            expect(
                bobSwapDetails.properties,
                "[Bob] should have same id as Alice"
            ).to.have.property("id", aliceToBobSwapDetails.properties.id);

            expect(
                [aliceToBobSwapDetails].map(swapDetail => toMatch(swapDetail))
            ).to.have.deep.members([toMatch(bobSwapDetails)]);
        });
    });
});
