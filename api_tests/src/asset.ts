import { Asset as AssetSdk } from "comit-sdk";
import { LedgerKind } from "./ledgers/ledger";

export interface Asset extends AssetSdk {
    name: AssetKind;
    ledger: LedgerKind;
    [k: string]: any;
}

export enum AssetKind {
    Bitcoin = "bitcoin",
    Ether = "ether",
    Erc20 = "erc20",
}

export function toKey(asset: Asset): string {
    return `${asset.name}-on-${asset.ledger}`;
}

export function toKind(key: string): { asset: AssetKind; ledger: LedgerKind } {
    switch (key) {
        case "bitcoin-on-bitcoin":
            return { asset: AssetKind.Bitcoin, ledger: LedgerKind.Bitcoin };
        case "bitcoin-on-lightning":
            return { asset: AssetKind.Bitcoin, ledger: LedgerKind.Lightning };
        case "ether-on-ethereum":
            return { asset: AssetKind.Ether, ledger: LedgerKind.Ethereum };
        case "erc20-on-ethereum":
            return { asset: AssetKind.Erc20, ledger: LedgerKind.Ethereum };
    }
}
