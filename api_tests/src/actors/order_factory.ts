export interface Herc20HbitOrder {
    trade: string;
    bitcoin_amount: string;
    bitcoin_ledger: string;
    ethereum_amount: string;
    token_contract: string;
    ethereum_ledger: Ethereum;
    absolute_expiry: number;
    refund_identity: string;
    redeem_identity: string;
}

interface Ethereum {
    chain_id: number;
}

export default class OrderFactory {
    public static newHerc20HbitOrder(): Herc20HbitOrder {
        return {
            trade: "sell",
            bitcoin_amount: "100000",
            bitcoin_ledger: "regtest",
            ethereum_amount: "200",
            token_contract: "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
            ethereum_ledger: {
                chain_id: 1337,
            },
            absolute_expiry: 600,
            refund_identity: "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX",
            redeem_identity: "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        };
    }
}
