import { Actor } from "./actor";
import { sleep } from "../utils";
import { HarnessGlobal } from "../utils";
import { defaultExpiries, getIdentities } from "./defaults";

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

export default class OrderbookFactory {
    public static async connect(alice: Actor, bob: Actor) {
        const addr = await bob.cnd.getPeerListenAddresses();
        // @ts-ignore
        await alice.cnd.client.post("dial", { addresses: addr });

        /// Wait for alice to accept an incoming connection from Bob
        await sleep(1000);
    }

    public static async initWalletsForBtcDaiOrder(alice: Actor, bob: Actor) {
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
