import { Herc20HbitPayload } from "../payload";

export interface Herc20HbitOrder {
    btc_quantity: string;
    bitcoin_ledger: string;
    erc20_token_contract: string;
    erc20_quantity: string;
    ethereum_ledger: Ethereum;
    alpha_expiry: number;
    beta_expiry: number;
    bob_refund_identity: string;
    bob_redeem_identity: string;
    maker_addr: string;
}

interface Ethereum {
    chain_id: number;
}

export default class OrderFactory {
    public static bobOrderFromSwap(
        makerAddr: string,
        swap: Herc20HbitPayload
    ): Herc20HbitOrder {
        return {
            btc_quantity: swap.alpha.amount,
            bitcoin_ledger: swap.beta.network,
            erc20_token_contract: swap.alpha.token_contract,
            erc20_quantity: swap.alpha.amount,
            ethereum_ledger: {
                chain_id: swap.alpha.chain_id,
            },
            alpha_expiry: swap.alpha.absolute_expiry,
            beta_expiry: swap.beta.absolute_expiry,
            bob_refund_identity: swap.beta.final_identity,
            bob_redeem_identity: swap.alpha.identity,
            maker_addr: makerAddr,
        };
    }
}
