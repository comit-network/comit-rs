import { Actor } from "./actor";
import { sleep } from "../utils";
import { HarnessGlobal } from "../utils";

declare var global: HarnessGlobal;

export interface Identities {
    refund_identity: string;
    redeem_identity: string;
}

export interface BtcDaiOrder {
    position: string;
    bitcoin_amount: string;
    bitcoin_ledger: string;
    ethereum_amount: string;
    token_contract: string;
    ethereum_ledger: Ethereum;
    bitcoin_absolute_expiry: number;
    ethereum_absolute_expiry: number;
    bitcoin_identity: string;
    ethereum_identity: string;
}

interface Ethereum {
    chain_id: number;
}

export default class OrderbookUtils {
    public static async initialiseWalletsForBtcDaiOrder(
        alice: Actor,
        bob: Actor
    ) {
        await alice.wallets.initializeForLedger(
            "bitcoin",
            alice.logger,
            "alice"
        );
        await alice.wallets.initializeForLedger(
            "ethereum",
            alice.logger,
            "alice"
        );

        await bob.wallets.initializeForLedger("bitcoin", bob.logger, "bob");
        await bob.wallets.initializeForLedger("ethereum", bob.logger, "bob");
    }

    public static async getIdentities(
        alice: Actor
    ): Promise<{ ethereum: string; lightning: string; bitcoin: string }> {
        return getIdentities(alice);
    }

    public static async connect(alice: Actor, bob: Actor) {
        // Get alice's listen address
        const aliceAddr = await alice.cnd.getPeerListenAddresses();

        // Bob dials alices
        // @ts-ignore
        await bob.cnd.client.post("dial", { addresses: aliceAddr });

        /// Wait for alice to accept an incoming connection from Bob
        await sleep(1000);
    }

    public static async newBtcDaiOrder(
        bob: Actor,
        position: string
    ): Promise<BtcDaiOrder> {
        const bobIdentities = await getIdentities(bob);

        // todo: do make this the actual DAI contract? It doesnt actually matter
        const daiTokenContract = global.tokenContract
            ? global.tokenContract
            : "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280";

        // todo: add a enum for buy/sell
        const expiries = function () {
            if (position === "buy") {
                return {
                    ethereum_absolute_expiry: defaultExpiries()
                        .betaAbsoluteExpiry,
                    bitcoin_absolute_expiry: defaultExpiries()
                        .alphaAbsoluteExpiry,
                };
            } else {
                return {
                    ethereum_absolute_expiry: defaultExpiries()
                        .alphaAbsoluteExpiry,
                    bitcoin_absolute_expiry: defaultExpiries()
                        .betaAbsoluteExpiry,
                };
            }
        };

        return {
            position,
            bitcoin_amount: "1000000",
            bitcoin_ledger: "regtest",
            token_contract: daiTokenContract,
            ethereum_amount: "9000000000000000000",
            ethereum_ledger: {
                chain_id: 1337,
            },
            ethereum_absolute_expiry: expiries().ethereum_absolute_expiry,
            bitcoin_absolute_expiry: expiries().bitcoin_absolute_expiry,
            bitcoin_identity: bobIdentities.bitcoin,
            ethereum_identity: bobIdentities.ethereum,
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

// async function makePeer(actor: Actor): Promise<Peer> {
//     return {
//         peer_id: await actor.cnd.getPeerId(),
//         address_hint: await actor.cnd
//             .getPeerListenAddresses()
//             .then((addresses) => addresses[0]),
//     };
// }

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
