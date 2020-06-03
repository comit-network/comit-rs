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
    HalightHerc20RequestBody,
    HalightRequestParams,
    HbitHerc20RequestBody,
    Herc20HalightRequestBody,
    Herc20HbitRequestBody,
    Herc20RequestParams,
    Peer,
} from "comit-sdk";
import { HarnessGlobal } from "../utils";
import { HbitRequestParams } from "comit-sdk/dist/src/cnd/swaps_payload";

declare var global: HarnessGlobal;

interface Ledgers {
    alpha: keyof AllWallets;
    beta: keyof AllWallets;
}

export default class SwapFactory {
    public static async newSwap(
        alice: Actor,
        bob: Actor,
        ledgers?: Ledgers
    ): Promise<{
        herc20Halight: {
            alice: Herc20HalightRequestBody;
            bob: Herc20HalightRequestBody;
        };
        halightHerc20: {
            alice: HalightHerc20RequestBody;
            bob: HalightHerc20RequestBody;
        };
        hbitHerc20: {
            alice: HbitHerc20RequestBody;
            bob: HbitHerc20RequestBody;
        };
        herc20Hbit: {
            alice: Herc20HbitRequestBody;
            bob: Herc20HbitRequestBody;
        };
    }> {
        const ledgerList = ledgers ? Object.values(ledgers) : [];
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
        } = defaultExpiries();

        const aliceIdentities = await getIdentities(alice);
        const bobIdentities = await getIdentities(bob);

        const herc20Hbit = {
            alice: {
                alpha: defaultHerc20RequestParams(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHbitRequestParams(
                    betaAbsoluteExpiry,
                    aliceIdentities.bitcoin
                ),
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHerc20RequestParams(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHbitRequestParams(
                    betaAbsoluteExpiry,
                    bobIdentities.bitcoin
                ),
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const hbitHerc20 = {
            alice: {
                alpha: defaultHbitRequestParams(
                    alphaAbsoluteExpiry,
                    aliceIdentities.bitcoin
                ),
                beta: defaultHerc20RequestParams(
                    betaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHbitRequestParams(
                    alphaAbsoluteExpiry,
                    bobIdentities.bitcoin
                ),
                beta: defaultHerc20RequestParams(
                    betaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const herc20Halight = {
            alice: {
                alpha: defaultHerc20RequestParams(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHalightRequestParams(
                    betaCltvExpiry,
                    aliceIdentities.lightning
                ),
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHerc20RequestParams(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHalightRequestParams(
                    betaCltvExpiry,
                    bobIdentities.lightning
                ),
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const halightHerc20 = {
            alice: {
                alpha: defaultHalightRequestParams(
                    alphaCltvExpiry,
                    aliceIdentities.lightning
                ),
                beta: defaultHerc20RequestParams(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),

                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHalightRequestParams(
                    alphaCltvExpiry,
                    bobIdentities.lightning
                ),
                beta: defaultHerc20RequestParams(
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
            herc20Halight,
            halightHerc20,
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

function defaultHbitRequestParams(
    absoluteExpiry: number,
    identity: string
): HbitRequestParams {
    return {
        amount: "1000000",
        network: "regtest",
        absolute_expiry: absoluteExpiry,
        identity,
    };
}

function defaultHalightRequestParams(
    cltvExpiry: number,
    lndPubkey: string
): HalightRequestParams {
    return {
        amount: "10000",
        network: "regtest",
        identity: lndPubkey,
        cltv_expiry: cltvExpiry,
    };
}

function defaultHerc20RequestParams(
    absoluteExpiry: number,
    identity: string,
    tokenContractAddress: string
): Herc20RequestParams {
    return {
        amount: "9000000000000000000",
        contract_address: tokenContractAddress,
        chain_id: 1337,
        identity,
        absolute_expiry: absoluteExpiry,
    };
}

function defaultExpiries() {
    const alphaAbsoluteExpiry = Math.round(Date.now() / 1000) + 240;
    const betaAbsoluteExpiry = Math.round(Date.now() / 1000) + 120;

    return {
        alphaAbsoluteExpiry,
        betaAbsoluteExpiry,
        alphaCltvExpiry: 350,
        betaCltvExpiry: 350,
    };
}
