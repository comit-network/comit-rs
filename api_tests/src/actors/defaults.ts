import { Ledger, LedgerKind } from "../ledgers/ledger";
import { Actor } from "./actor";

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
                chain_id: 1337,
            };
        }
    }
}

export async function getIdentities(
    self: Actor
): Promise<{ ethereum: string; lightning: string; bitcoin: string }> {
    let ethereum = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
    let lightning =
        "02ed138aaed50d2d597f6fe8d30759fd3949fe73fdf961322713f1c19e10036a06";
    let bitcoin =
        "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275";

    try {
        ethereum = self.wallets.ethereum.account();
    } catch (e) {
        self.logger.warn(
            "Ethereum wallet not available, using static value for identity"
        );
    }

    try {
        lightning = await self.wallets.lightning.inner.getPubkey();
    } catch (e) {
        self.logger.warn(
            "Lightning wallet not available, using static value for identity"
        );
    }

    try {
        bitcoin = await self.wallets.bitcoin.address();
    } catch (e) {
        self.logger.warn(
            "Bitcoin wallet not available, using static value for identity"
        );
    }

    return {
        ethereum,
        lightning,
        bitcoin,
    };
}

export function defaultExpiries() {
    const {
        alphaAbsoluteExpiry,
        betaAbsoluteExpiry,
        alphaCltvExpiry,
        betaCltvExpiry,
    } = nowExpiries();

    return {
        alphaAbsoluteExpiry: alphaAbsoluteExpiry + 240,
        betaAbsoluteExpiry: betaAbsoluteExpiry + 120,
        alphaCltvExpiry,
        betaCltvExpiry,
    };
}

export function nowExpiries() {
    const alphaAbsoluteExpiry = Math.round(Date.now() / 1000);
    const betaAbsoluteExpiry = Math.round(Date.now() / 1000);

    return {
        alphaAbsoluteExpiry,
        betaAbsoluteExpiry,
        alphaCltvExpiry: 350,
        betaCltvExpiry: 350,
    };
}
