import { Herc20HbitPayload } from "../payload";

export interface Herc20HbitOrder {
    position: string;
    bitcoin_amount: string;
    bitcoin_ledger: string;
    ethereum_amount: string;
    token_contract: string;
    ethereum_ledger: Ethereum;
    bitcoin_absolute_expiry: number;
    ethereum_absolute_expiry: number;
    refund_identity: string;
    redeem_identity: string;
}

interface Ethereum {
    chain_id: number;
}

export default class OrderFactory {
    public static newHerc20HbitSellOrder(
        swap: Herc20HbitPayload
    ): Herc20HbitOrder {
        return {
            position: "sell",
            bitcoin_amount: swap.alpha.amount,
            bitcoin_ledger: swap.beta.network,
            token_contract: swap.alpha.token_contract,
            ethereum_amount: swap.alpha.amount,
            ethereum_ledger: {
                chain_id: swap.alpha.chain_id,
            },
            ethereum_absolute_expiry: swap.alpha.absolute_expiry,
            bitcoin_absolute_expiry: swap.beta.absolute_expiry,
            refund_identity: swap.beta.final_identity,
            redeem_identity: swap.alpha.identity,
        };
    }
}
