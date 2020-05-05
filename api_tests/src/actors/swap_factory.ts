/*
 * Creates swaps for the given actors.
 *
 * In order for cnd to successfully execute swaps, the parameters (expiry-times etc) need to exactly match.
 * Hence, we generate this data in one single place.
 * The swap factory is this place.
 *
 * It is a replacement for a negotiation/order protocol that takes care of this in a real application.
 */
import { Actor } from "./actor";
import {
    AllWallets,
    HalightLightningBitcoinHanEthereumEtherRequestBody,
    HalightLightningBitcoinHerc20EthereumErc20RequestBody,
    HalightLightningBitcoinRequestParams,
    HanEthereumEtherHalightLightningBitcoinRequestBody,
    HanEthereumEtherRequestParams,
    Herc20EthereumErc20HalightLightningBitcoinRequestBody,
    Herc20EthereumErc20RequestParams,
    Peer,
} from "comit-sdk";

export default class SwapFactory {
    public static async newSwap(
        alice: Actor,
        bob: Actor
    ): Promise<
        [
            HanEthereumEtherHalightLightningBitcoinRequestBody,
            HanEthereumEtherHalightLightningBitcoinRequestBody
        ]
    > {
        const ledgers: (keyof AllWallets)[] = ["ethereum", "lightning"];

        for (const ledger of ledgers) {
            await alice.wallets.initializeForLedger(
                ledger,
                alice.logger,
                "alice"
            );
            await bob.wallets.initializeForLedger(ledger, bob.logger, "bob");
        }

        const {
            alphaAbsoluteExpiry,
            betaCltvExpiry,
        } = defaultHalightHanHerc20Expiries();

        const aliceCreateSwapBody = await makeCreateSwapBody(
            alice,
            "Alice",
            bob,
            alphaAbsoluteExpiry,
            betaCltvExpiry
        );
        const bobCreateSwapBody = await makeCreateSwapBody(
            bob,
            "Bob",
            alice,
            alphaAbsoluteExpiry,
            betaCltvExpiry
        );

        return [aliceCreateSwapBody, bobCreateSwapBody];
    }
}

async function makeCreateSwapBody(
    self: Actor,
    cryptoRole: "Alice" | "Bob",
    counterparty: Actor,
    alphaAbsoluteExpiry: number,
    betaCltvExpiry: number
): Promise<HanEthereumEtherHalightLightningBitcoinRequestBody> {
    return {
        alpha: defaultHanEthereumEtherRequestParams(
            alphaAbsoluteExpiry,
            self.wallets.ethereum.account()
        ),
        beta: defaultHalightLightningBitcoinRequestParams(
            betaCltvExpiry,
            await self.wallets.lightning.inner.getPubkey()
        ),
        role: cryptoRole,
        peer: await makePeer(counterparty),
    };
}

async function makePeer(actor: Actor): Promise<Peer> {
    return {
        peer_id: await actor.cnd.getPeerId(),
        address_hint: await actor.cnd
            .getPeerListenAddresses()
            .then((addresses) => addresses[0]),
    };
}

function defaultHanEthereumEtherRequestParams(
    absoluteExpiry: number,
    identity: string
): HanEthereumEtherRequestParams {
    return {
        amount: "5000000000000000000",
        chain_id: 17,
        identity,
        absolute_expiry: absoluteExpiry,
    };
}

function defaultHalightLightningBitcoinRequestParams(
    cltvExpiry: number,
    lndPubkey: string
): HalightLightningBitcoinRequestParams {
    return {
        amount: "10000",
        network: "regtest",
        identity: lndPubkey,
        cltv_expiry: cltvExpiry,
    };
}

function defaultHerc20EthereumErc20RequestParams(
    absoluteExpiry: number
): Herc20EthereumErc20RequestParams {
    return {
        amount: "9000000000000000000",
        contract_address: "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
        chain_id: 17,
        identity: "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        absolute_expiry: absoluteExpiry,
    };
}
export function defaultHanEthereumEtherHalightLightningBitcoin(
    lndPubkey: string,
    peer: Peer,
    role: "Alice" | "Bob",
    ethereumIdentity: string
): HanEthereumEtherHalightLightningBitcoinRequestBody {
    const {
        alphaAbsoluteExpiry,
        betaCltvExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHanEthereumEtherRequestParams(
            alphaAbsoluteExpiry,
            ethereumIdentity
        ),
        beta: defaultHalightLightningBitcoinRequestParams(
            betaCltvExpiry,
            lndPubkey
        ),
        role,
        peer,
    };
}

export function defaultHerc20EthereumErc20HalightLightningBitcoin(
    lndPubkey: string,
    peer: Peer
): Herc20EthereumErc20HalightLightningBitcoinRequestBody {
    const {
        alphaAbsoluteExpiry,
        betaCltvExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHerc20EthereumErc20RequestParams(alphaAbsoluteExpiry),
        beta: defaultHalightLightningBitcoinRequestParams(
            betaCltvExpiry,
            lndPubkey
        ),
        role: "Alice",
        peer,
    };
}

export function defaultHalightLightningBitcoinHanEthereumEther(
    lndPubkey: string,
    peer: Peer,
    ethereumIdentity: string
): HalightLightningBitcoinHanEthereumEtherRequestBody {
    const {
        alphaCltvExpiry,
        betaAbsoluteExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHalightLightningBitcoinRequestParams(
            alphaCltvExpiry,
            lndPubkey
        ),
        beta: defaultHanEthereumEtherRequestParams(
            betaAbsoluteExpiry,
            ethereumIdentity
        ),
        role: "Alice",
        peer,
    };
}

export function defaultHalightLightningBitcoinHerc20EthereumErc20(
    lndPubkey: string,
    peer: Peer
): HalightLightningBitcoinHerc20EthereumErc20RequestBody {
    const {
        alphaCltvExpiry,
        betaAbsoluteExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHalightLightningBitcoinRequestParams(
            alphaCltvExpiry,
            lndPubkey
        ),
        beta: defaultHerc20EthereumErc20RequestParams(betaAbsoluteExpiry),
        role: "Alice",
        peer,
    };
}

function defaultHalightHanHerc20Expiries() {
    const alphaAbsoluteExpiry = Math.round(Date.now() / 1000) + 8;
    const betaAbsoluteExpiry = Math.round(Date.now() / 1000) + 3;

    return {
        alphaAbsoluteExpiry,
        betaAbsoluteExpiry,
        alphaCltvExpiry: 350,
        betaCltvExpiry: 350,
    };
}
