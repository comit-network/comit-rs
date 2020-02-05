import { ethers } from "ethers";
import { Actor } from "../lib_sdk/actors/actor";
import { Cnd, SwapRequest } from "comit-sdk";
import { Entity } from "../gen/siren";
import { expect } from "chai";
import { sleep } from "../lib_sdk/utils";
import { SwapDetails } from "comit-sdk/dist/src/cnd";

export const DEFAULT_ALPHA = {
    ledger: {
        name: "bitcoin",
        network: "regtest",
    },
    asset: {
        name: "bitcoin",
        quantity: {
            bob: "100000000",
            charlie: "200000000",
            reasonable: "100000000",
            stingy: "100",
        },
    },
    expiry: new Date("2080-06-11T23:00:00Z").getTime() / 1000,
};

const DEFAULT_BETA = {
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
export async function createDefaultSwapRequest(counterParty: Actor) {
    const swapRequest: SwapRequest = {
        alpha_ledger: {
            name: DEFAULT_ALPHA.ledger.name,
            network: DEFAULT_ALPHA.ledger.network,
        },
        beta_ledger: {
            name: DEFAULT_BETA.ledger.name,
            chain_id: DEFAULT_BETA.ledger.chain_id,
        },
        alpha_asset: {
            name: DEFAULT_ALPHA.asset.name,
            quantity: DEFAULT_ALPHA.asset.quantity.bob,
        },
        beta_asset: {
            name: DEFAULT_BETA.asset.name,
            quantity: DEFAULT_BETA.asset.quantity.bob,
        },
        beta_ledger_redeem_identity:
            "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        alpha_expiry: DEFAULT_ALPHA.expiry,
        beta_expiry: DEFAULT_BETA.expiry,
        peer: {
            peer_id: await counterParty.cnd.getPeerId(),
            address_hint: await counterParty.cnd
                .getPeerListenAddresses()
                .then(addresses => addresses[0]),
        },
    };
    return swapRequest;
}

export async function pollCndUntil(
    cnd: Cnd,
    location: string,
    predicate: (body: Entity) => boolean
): Promise<Entity> {
    const response = await cnd.fetch(location);

    expect(response).to.have.status(200);

    if (predicate(response.data)) {
        return response.data;
    } else {
        await sleep(500);

        return this.pollCndUntil(location, predicate);
    }
}

export async function pollSwapDetails(
    cnd: Cnd,
    swapUrl: string,
    iteration: number = 0
): Promise<SwapDetails> {
    if (iteration > 5) {
        throw new Error(`Could not retrieve Swap ${swapUrl}`);
    }
    iteration++;

    try {
        return (await cnd.fetch<SwapDetails>(swapUrl)).data;
    } catch (error) {
        await sleep(1000);
        return await pollSwapDetails(cnd, swapUrl, iteration);
    }
}
