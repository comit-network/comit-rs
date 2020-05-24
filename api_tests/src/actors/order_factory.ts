export interface Herc20HbitOrder {
    buy_quantity: string;
    sell_token_contract: string;
    sell_quantity: string;
    absolute_expiry: number;
    refund_identity: string;
    redeem_identity: string;
    maker_addr: string;
}

export default class OrderFactory {
    public static newHerc20HbitOrder(makerAddr: string): Herc20HbitOrder {
        return {
            buy_quantity: "100",
            sell_token_contract: "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
            sell_quantity: "200",
            absolute_expiry: 600,
            refund_identity: "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX",
            redeem_identity: "0x00a329c0648769a73afac7f9381e08fb43dbea72",
            maker_addr: makerAddr,
        };
    }
}
