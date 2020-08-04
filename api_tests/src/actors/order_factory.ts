import { Actor } from "./actor";
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
    rate: string;
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

export function ethereumAmountInWei(order: BtcDaiOrder): bigint {
    return rate(order) * bitcoinAmountInSatoshi(order.bitcoin_amount);
}

export function rate(order: BtcDaiOrder): bigint {
    const precision = 10;
    if (order.rate.split(".").length !== 2) {
        throw new Error("rate contains more than 1 decimal point");
    }
    const integer = order.rate.split(".")[0];
    const decimals = order.rate.split(".")[1];
    const trailingZeroes = "0".repeat(precision - decimals.length);

    const result = integer.concat(decimals).concat(trailingZeroes);
    return BigInt(result);
}

export function bitcoinAmountInSatoshi(bitcoinAmount: string): bigint {
    const precision = 8;
    const parts = bitcoinAmount.split(".");
    if (parts.length !== 2) {
        throw new Error("rate contains more than 1 decimal point");
    }
    const integer = parts[0];
    const decimals = parts[1];
    const trailingZeroes = "0".repeat(precision - decimals.length);

    const result = integer.concat(decimals).concat(trailingZeroes);
    return BigInt(result);
}

export default class OrderbookFactory {
    public static async newBtcDaiOrder(
        alice: Actor,
        bob: Actor,
        position: string,
        rate: string,
        amount: string
    ): Promise<BtcDaiOrder> {
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

        const order = {
            position,
            bitcoin_amount: amount,
            bitcoin_ledger: "regtest",
            token_contract: daiTokenContract,
            rate,
            ethereum_ledger: {
                chain_id: 1337,
            },
            ethereum_absolute_expiry: expiries().ethereum_absolute_expiry,
            bitcoin_absolute_expiry: expiries().bitcoin_absolute_expiry,
            bitcoin_identity: bobIdentities.bitcoin,
            ethereum_identity: bobIdentities.ethereum,
        };

        await alice.initLedgersForOrder(order);
        await bob.initLedgersForOrder(order);

        return order;
    }
}
