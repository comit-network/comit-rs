export interface Herc20HbitOrder {
    buy_quantity: string;
    bitcoin_ledger: string;
    sell_token_contract: string;
    sell_quantity: string;
    ethereum_ledger: Ethereum;
    absolute_expiry: number;
    refund_identity: string;
    redeem_identity: string;
    maker_addr: string;
}

interface Ethereum {
    chain_id: number;
}

export default class OrderFactory {
    public static newHerc20HbitOrder(makerAddr: string): Herc20HbitOrder {
        return {
            buy_quantity: "100",
            bitcoin_ledger: "regtest",
            sell_token_contract: "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
            sell_quantity: "200",
            ethereum_ledger: {
                chain_id: 1337,
            },
            absolute_expiry: 600,
            refund_identity: "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX",
            redeem_identity: "0x00a329c0648769a73afac7f9381e08fb43dbea72",
            maker_addr: makerAddr,
        };
    }
}
