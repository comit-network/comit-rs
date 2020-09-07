import { LedgerKind } from "./ledger";
import { parseEther } from "ethers/lib/utils";
import { HarnessGlobal } from "./environment";

declare var global: HarnessGlobal;

export interface Asset {
    name: AssetKind;
    ledger: LedgerKind;
    tokenContract?: string;
    quantity: string;
}

export enum AssetKind {
    Bitcoin = "bitcoin",
    Ether = "ether",
    Erc20 = "erc20",
}

export type assetAsKey = string;

export function toKey(asset: Asset): assetAsKey {
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

export function defaultAssetValue(asset: AssetKind, ledger: LedgerKind): Asset {
    switch (asset) {
        case AssetKind.Bitcoin: {
            return {
                name: AssetKind.Bitcoin,
                ledger,
                quantity: "10000000",
            };
        }
        case AssetKind.Ether: {
            return {
                name: AssetKind.Ether,
                ledger,
                quantity: parseEther("10").toString(),
            };
        }
        case AssetKind.Erc20: {
            return {
                name: AssetKind.Erc20,
                ledger,
                quantity: parseEther("100").toString(),
                tokenContract: global.tokenContract,
            };
        }
    }
}
