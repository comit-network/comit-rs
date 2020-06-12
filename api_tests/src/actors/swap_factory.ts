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
import { AllWallets, Peer } from "comit-sdk";
import {
    HalbitHerc20Payload,
    Herc20HalbitPayload,
    HbitHerc20Payload,
    Herc20HbitPayload,
    HalbitPayload,
    Herc20Payload,
    HbitPayload,
} from "../payload";
import { HarnessGlobal } from "../utils";

declare var global: HarnessGlobal;

interface SwapSettings {
    ledgers?: { alpha: keyof AllWallets; beta: keyof AllWallets };
    instantRefund?: boolean;
}

export default class SwapFactory {
    public static async newSwap(
        alice: Actor,
        bob: Actor,
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
        const ledgerList = settings.ledgers
            ? Object.values(settings.ledgers)
            : [];
        for (const ledger of ledgerList) {
            await alice.wallets.initializeForLedger(
                ledger,
                alice.logger,
                "alice"
            );
            await bob.wallets.initializeForLedger(ledger, bob.logger, "bob");
        }

        const erc20TokenContract = global.tokenContract
            ? global.tokenContract
            : "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280";

        const {
            alphaAbsoluteExpiry,
            betaAbsoluteExpiry,
            alphaCltvExpiry,
            betaCltvExpiry,
        } = settings.instantRefund ? nowExpiries() : defaultExpiries();

        const aliceIdentities = await getIdentities(alice);
        const bobIdentities = await getIdentities(bob);

        const herc20Hbit = {
            alice: {
                alpha: defaultHerc20Payload(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHbitPayload(
                    betaAbsoluteExpiry,
                    aliceIdentities.bitcoin
                ),
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHerc20Payload(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHbitPayload(
                    betaAbsoluteExpiry,
                    bobIdentities.bitcoin
                ),
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const hbitHerc20 = {
            alice: {
                alpha: defaultHbitPayload(
                    alphaAbsoluteExpiry,
                    aliceIdentities.bitcoin
                ),
                beta: defaultHerc20Payload(
                    betaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHbitPayload(
                    alphaAbsoluteExpiry,
                    bobIdentities.bitcoin
                ),
                beta: defaultHerc20Payload(
                    betaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const herc20Halbit = {
            alice: {
                alpha: defaultHerc20Payload(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHalbitPayload(
                    betaCltvExpiry,
                    aliceIdentities.lightning
                ),
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHerc20Payload(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHalbitPayload(
                    betaCltvExpiry,
                    bobIdentities.lightning
                ),
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const halbitHerc20 = {
            alice: {
                alpha: defaultHalbitPayload(
                    alphaCltvExpiry,
                    aliceIdentities.lightning
                ),
                beta: defaultHerc20Payload(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),

                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHalbitPayload(
                    alphaCltvExpiry,
                    bobIdentities.lightning
                ),
                beta: defaultHerc20Payload(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),

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

async function getIdentities(
    self: Actor
): Promise<{ ethereum: string; lightning: string; bitcoin: string }> {
    let ethereum = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    let lightning =
        "02ed138aaed50d2d597f6fe8d30759fd3949fe73fdf961322713f1c19e10036a06";
    let bitcoin =
        "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275";

    try {
        ethereum = self.wallets.ethereum.account();
    } catch (e) {
        self.logger.warn(
            "Ethereum wallet not available, using static value for identity"
        );
    }

    try {
        lightning = await self.wallets.lightning.inner.getPubkey();
    } catch (e) {
        self.logger.warn(
            "Lightning wallet not available, using static value for identity"
        );
    }

    try {
        bitcoin = await self.wallets.bitcoin.address();
    } catch (e) {
        self.logger.warn(
            "Bitcoin wallet not available, using static value for identity"
        );
    }

    return {
        ethereum,
        lightning,
        bitcoin,
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

function defaultHbitPayload(
    absoluteExpiry: number,
    finalIdentity: string
): HbitPayload {
    return {
        amount: "1000000",
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
        amount: "10000",
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
        amount: "9000000000000000000",
        token_contract: tokenContract,
        chain_id: 1337,
        identity,
        absolute_expiry: absoluteExpiry,
    };
}

function defaultExpiries() {
    const {
        alphaAbsoluteExpiry,
        betaAbsoluteExpiry,
        alphaCltvExpiry,
        betaCltvExpiry,
    } = nowExpiries();

    return {
        alphaAbsoluteExpiry: alphaAbsoluteExpiry + 240,
        betaAbsoluteExpiry: betaAbsoluteExpiry + 120,
        alphaCltvExpiry,
        betaCltvExpiry,
    };
}

function nowExpiries() {
    const alphaAbsoluteExpiry = Math.round(Date.now() / 1000);
    const betaAbsoluteExpiry = Math.round(Date.now() / 1000);

    return {
        alphaAbsoluteExpiry,
        betaAbsoluteExpiry,
        alphaCltvExpiry: 350,
        betaCltvExpiry: 350,
    };
}
