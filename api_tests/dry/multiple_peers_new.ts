// These are stateless tests -- they don't require any state of the cnd and they don't change it
// They are mostly about checking invalid request responses
// These test do not use the sdk so that we can test edge cases
import { threeActorTest } from "../lib_sdk/actor_test";
import { request } from "chai";
import "chai/register-should";
import { ethers } from "ethers";
import "../lib/setup_chai";
import { Actor } from "../lib_sdk/actors/actor";

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

async function createDefaultSwapRequest(bob: Actor) {
    return {
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
        peer: await bob.cnd.getPeerId(),
    };
}

setTimeout(async function() {
    describe("SWAP requests to multiple peers", () => {
        threeActorTest(
            "[Alice] Should be able to send a swap request to Bob",
            async function({ alice, bob }) {
                const res = await request(alice.cndHttpApiUrl())
                    .post("/swaps/rfc003")
                    .send(await createDefaultSwapRequest(bob));

                res.status.should.equal(201);
                res.error.should.equal(false);

                const aliceSwapWithBobHref = res.header.location;
                aliceSwapWithBobHref.should.be.a("string");
            }
        );

        // threeActorTest(
        //     "[Bob] should use the same swap id as Alice",
        //     async function({ alice, bob }) {
        //         const aliceResponse = await request(alice.cndHttpApiUrl())
        //             .post("/swaps/rfc003")
        //             .send(await createDefaultSwapRequest(bob));
        //
        //         const aliceSwapId = aliceResponse.body.properties.id;
        //
        //         const bobSwap = await bob.pollUntilSwapAvailable();
        //
        //         expect(bobSwap.properties).to.have.property("id", aliceSwapId);
        //
        //     }
        // );
    });

    run();
}, 0);
