/*
 * Creates swaps for the given actors.
 *
 * In order for cnd to successfully execute swaps, the parameters (expiry-times etc) need to exactly match.
 * Hence, we generate this data in one single place.
 * The swap factory is this place.
 *
 * It is a replacement for a negotiation/order protocol that takes care of this in a real application.
 */
import { CndActor } from "./actors/cnd_actor";
import {
    HalbitHerc20Payload,
    HalbitPayload,
    HbitHerc20Payload,
    HbitPayload,
    Herc20HalbitPayload,
    Herc20HbitPayload,
    Herc20Payload,
    Peer,
} from "./cnd_client/payload";
import { defaultExpiries, nowExpiries } from "./actors/defaults";
import { HarnessGlobal } from "./environment";

declare var global: HarnessGlobal;

interface SwapSettings {
    instantRefund?: boolean;
}

export default class SwapFactory {
    public static async newSwap(
        alice: CndActor,
        bob: CndActor,
        settings: SwapSettings = { instantRefund: false }
    ): Promise<{
        herc20Halbit: {
            alice: Herc20HalbitPayload;
            bob: Herc20HalbitPayload;
        };
        halbitHerc20: {
            alice: HalbitHerc20Payload;
            bob: HalbitHerc20Payload;
        };
        hbitHerc20: {
            alice: HbitHerc20Payload;
            bob: HbitHerc20Payload;
        };
        herc20Hbit: {
            alice: Herc20HbitPayload;
            bob: Herc20HbitPayload;
        };
    }> {
        const erc20TokenContract = global.tokenContract
            ? global.tokenContract
            : "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280";

        const {
            alphaAbsoluteExpiry,
            betaAbsoluteExpiry,
            alphaCltvExpiry,
            betaCltvExpiry,
        } = settings.instantRefund ? nowExpiries() : defaultExpiries();

        const aliceEthereumAccount = alice.wallets.ethereum.getAccount();
        const aliceBitcoinAddress = await alice.wallets.bitcoin.getAddress();
        const aliceLightningPubkey = await alice.lndClient.getPubkey();

        const bobEthereumAccount = bob.wallets.ethereum.getAccount();
        const bobBitcoinAddress = await bob.wallets.bitcoin.getAddress();
        const bobLightningPubkey = await bob.lndClient.getPubkey();

        const aliceAlphaHerc20 = defaultHerc20Payload(
            alphaAbsoluteExpiry,
            aliceEthereumAccount,
            erc20TokenContract
        );
        const aliceBetaHbit = defaultHbitPayload(
            betaAbsoluteExpiry,
            aliceBitcoinAddress
        );
        const bobAlphaHerc20 = defaultHerc20Payload(
            alphaAbsoluteExpiry,
            bobEthereumAccount,
            erc20TokenContract
        );
        const bobBetaHbit = defaultHbitPayload(
            betaAbsoluteExpiry,
            bobBitcoinAddress
        );
        const aliceAlphaHbit = defaultHbitPayload(
            alphaAbsoluteExpiry,
            aliceBitcoinAddress
        );
        const aliceBetaHerc20 = defaultHerc20Payload(
            betaAbsoluteExpiry,
            aliceEthereumAccount,
            erc20TokenContract
        );
        const bobAlphaHbit = defaultHbitPayload(
            alphaAbsoluteExpiry,
            bobBitcoinAddress
        );
        const bobBetaHerc20 = defaultHerc20Payload(
            betaAbsoluteExpiry,
            bobEthereumAccount,
            erc20TokenContract
        );
        const aliceBetaHalbit = defaultHalbitPayload(
            betaCltvExpiry,
            aliceLightningPubkey
        );
        const bobBetaHalbit = defaultHalbitPayload(
            betaCltvExpiry,
            bobLightningPubkey
        );
        const aliceAlphaHalbit = defaultHalbitPayload(
            alphaCltvExpiry,
            aliceLightningPubkey
        );
        const bobAlphaHalbit = defaultHalbitPayload(
            alphaCltvExpiry,
            bobLightningPubkey
        );

        const herc20Hbit = {
            alice: {
                alpha: aliceAlphaHerc20,
                beta: aliceBetaHbit,
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: bobAlphaHerc20,
                beta: bobBetaHbit,
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };
        const hbitHerc20 = {
            alice: {
                alpha: aliceAlphaHbit,
                beta: aliceBetaHerc20,
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: bobAlphaHbit,
                beta: bobBetaHerc20,
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };
        const herc20Halbit = {
            alice: {
                alpha: aliceAlphaHerc20,
                beta: aliceBetaHalbit,
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: bobAlphaHerc20,
                beta: bobBetaHalbit,
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };
        const halbitHerc20 = {
            alice: {
                alpha: aliceAlphaHalbit,
                beta: aliceBetaHerc20,

                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: bobAlphaHalbit,
                beta: bobBetaHerc20,

                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        return {
            hbitHerc20,
            herc20Hbit,
            herc20Halbit,
            halbitHerc20,
        };
    }
}

async function makePeer(actor: CndActor): Promise<Peer> {
    return {
        peer_id: await actor.cnd.getPeerId(),
        address_hint: await actor.cnd
            .getPeerListenAddresses()
            .then((addresses) => addresses[0]),
    };
}

function defaultHbitPayload(
    absoluteExpiry: number,
    finalIdentity: string
): HbitPayload {
    return {
        amount: 1000000n,
        final_identity: finalIdentity,
        network: "regtest",
        absolute_expiry: absoluteExpiry,
    };
}

function defaultHalbitPayload(
    cltvExpiry: number,
    lndPubkey: string
): HalbitPayload {
    return {
        amount: 100000n,
        network: "regtest",
        identity: lndPubkey,
        cltv_expiry: cltvExpiry,
    };
}

function defaultHerc20Payload(
    absoluteExpiry: number,
    identity: string,
    tokenContract: string
): Herc20Payload {
    return {
        amount: 9000000000000000000n,
        token_contract: tokenContract,
        chain_id: 1337,
        identity,
        absolute_expiry: absoluteExpiry,
    };
}
