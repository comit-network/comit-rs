import { Ledger as LedgerSdk } from "comit-sdk";

export interface Ledger extends LedgerSdk {
    name: LedgerKind;
    [k: string]: any;
}

export enum LedgerKind {
    Bitcoin = "bitcoin",
    Ethereum = "ethereum",
}
