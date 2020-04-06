import { HarnessGlobal } from "../utils";
import { Asset, AssetKind } from "../asset";
import { Ledger, LedgerKind } from "../ledgers/ledger";
import { parseEther } from "ethers/utils";

declare var global: HarnessGlobal;

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
 * WIP as the cnd REST API routes for lightning are not yet defined.
 * @param ledger
 * @returns The ledger formatted as needed for the request body to cnd HTTP API on the lightning route.
 */
export function defaultLedgerDescriptionForLedger(ledger: LedgerKind): Ledger {
    switch (ledger) {
        case LedgerKind.Lightning: {
            return {
                name: LedgerKind.Lightning,
            };
        }
        case LedgerKind.Bitcoin: {
            return {
                name: LedgerKind.Bitcoin,
                network: "regtest",
            };
        }
        case LedgerKind.Ethereum: {
            return {
                name: LedgerKind.Ethereum,
                chain_id: 17,
            };
        }
    }
}

export function defaultAssetDescription(
    asset: AssetKind,
    ledger: LedgerKind
): Asset {
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
                token_contract: global.tokenContract,
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
