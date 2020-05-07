import { AssetKind } from "../asset";
import { Ledger, LedgerKind } from "../ledgers/ledger";

export function defaultLedgerKindForAsset(asset: AssetKind): LedgerKind {
    switch (asset) {
        case AssetKind.Bitcoin:
            return LedgerKind.Bitcoin;
        case AssetKind.Ether:
            return LedgerKind.Ethereum;
        case AssetKind.Erc20:
            return LedgerKind.Ethereum;
    }
}

/**
 * @param ledger
 * @returns The ledger formatted as needed for the request body to cnd HTTP API on the lightning route.
 */
export function defaultLedgerDescriptionForLedger(ledger: LedgerKind): Ledger {
    switch (ledger) {
        case LedgerKind.Bitcoin: {
            return {
                name: LedgerKind.Bitcoin,
                network: "regtest",
            };
        }
        case LedgerKind.Ethereum: {
            return {
                name: LedgerKind.Ethereum,
                chain_id: 1337,
            };
        }
    }
}

export function defaultExpiryTimes() {
    const alphaExpiry = Math.round(Date.now() / 1000) + 8;
    const betaExpiry = Math.round(Date.now() / 1000) + 3;

    return {
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
    };
}
