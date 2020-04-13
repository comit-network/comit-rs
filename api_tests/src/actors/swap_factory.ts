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
import { HarnessGlobal } from "../utils";

declare var global: HarnessGlobal;

export default class SwapFactory {
    public static async newSwap(
        alice: Actor,
        bob: Actor,
        dry?: boolean
    ): Promise<{
        hanEthereumEtherHalightLightningBitcoin: {
            alice: HanEthereumEtherHalightLightningBitcoinRequestBody;
            bob: HanEthereumEtherHalightLightningBitcoinRequestBody;
        };
        herc20EthereumErc20HalightLightningBitcoin: {
            alice: Herc20EthereumErc20HalightLightningBitcoinRequestBody;
            bob: Herc20EthereumErc20HalightLightningBitcoinRequestBody;
        };
        halightLightningBitcoinHanEthereumEther: {
            alice: HalightLightningBitcoinHanEthereumEtherRequestBody;
            bob: HalightLightningBitcoinHanEthereumEtherRequestBody;
        };
        halightLightningBitcoinHerc20EthereumErc20: {
            alice: HalightLightningBitcoinHerc20EthereumErc20RequestBody;
            bob: HalightLightningBitcoinHerc20EthereumErc20RequestBody;
        };
    }> {
        const ledgers: (keyof AllWallets)[] = dry
            ? []
            : ["ethereum", "lightning"];

        for (const ledger of ledgers) {
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
            betaCltvExpiry,
        } = defaultHalightHanHerc20Expiries();

        const aliceIdentities = await getIdentities(alice);
        const bobIdentities = await getIdentities(bob);

        const hanEthereumEtherHalightLightningBitcoin = {
            alice: {
                alpha: defaultHanEthereumEtherRequestParams(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum
                ),
                beta: defaultHalightLightningBitcoinRequestParams(
                    betaCltvExpiry,
                    aliceIdentities.lightning
                ),
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHanEthereumEtherRequestParams(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum
                ),
                beta: defaultHalightLightningBitcoinRequestParams(
                    betaCltvExpiry,
                    bobIdentities.lightning
                ),
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const herc20EthereumErc20HalightLightningBitcoin = {
            alice: {
                alpha: defaultHerc20EthereumErc20RequestParams(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHalightLightningBitcoinRequestParams(
                    betaCltvExpiry,
                    aliceIdentities.lightning
                ),
                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHerc20EthereumErc20RequestParams(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),
                beta: defaultHalightLightningBitcoinRequestParams(
                    betaCltvExpiry,
                    bobIdentities.lightning
                ),
                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const halightLightningBitcoinHanEthereumEther = {
            alice: {
                alpha: defaultHalightLightningBitcoinRequestParams(
                    betaCltvExpiry,
                    aliceIdentities.lightning
                ),
                beta: defaultHanEthereumEtherRequestParams(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum
                ),

                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHalightLightningBitcoinRequestParams(
                    betaCltvExpiry,
                    bobIdentities.lightning
                ),
                beta: defaultHanEthereumEtherRequestParams(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum
                ),

                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        const halightLightningBitcoinHerc20EthereumErc20 = {
            alice: {
                alpha: defaultHalightLightningBitcoinRequestParams(
                    betaCltvExpiry,
                    aliceIdentities.lightning
                ),
                beta: defaultHerc20EthereumErc20RequestParams(
                    alphaAbsoluteExpiry,
                    aliceIdentities.ethereum,
                    erc20TokenContract
                ),

                role: "Alice" as "Alice" | "Bob",
                peer: await makePeer(bob),
            },
            bob: {
                alpha: defaultHalightLightningBitcoinRequestParams(
                    betaCltvExpiry,
                    bobIdentities.lightning
                ),
                beta: defaultHerc20EthereumErc20RequestParams(
                    alphaAbsoluteExpiry,
                    bobIdentities.ethereum,
                    erc20TokenContract
                ),

                role: "Bob" as "Alice" | "Bob",
                peer: await makePeer(alice),
            },
        };

        return {
            hanEthereumEtherHalightLightningBitcoin,
            herc20EthereumErc20HalightLightningBitcoin,
            halightLightningBitcoinHanEthereumEther,
            halightLightningBitcoinHerc20EthereumErc20,
        };
    }
}

async function getIdentities(
    self: Actor
): Promise<{ ethereum: string; lightning: string }> {
    let ethereum = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    let lightning =
        "02ed138aaed50d2d597f6fe8d30759fd3949fe73fdf961322713f1c19e10036a06";

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

    return {
        ethereum,
        lightning,
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
    absoluteExpiry: number,
    identity: string,
    tokenContractAddress: string
): Herc20EthereumErc20RequestParams {
    return {
        amount: "9000000000000000000",
        contract_address: tokenContractAddress,
        chain_id: 17,
        identity,
        absolute_expiry: absoluteExpiry,
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
