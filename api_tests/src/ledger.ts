export interface Ledger {
    name: LedgerKind;
    [k: string]: any;
}

export enum LedgerKind {
    Bitcoin = "bitcoin",
    Ethereum = "ethereum",
    Lightning = "lightning",
}
