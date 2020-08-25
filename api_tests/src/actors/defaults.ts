import { Ledger, LedgerKind } from "../ledgers/ledger";

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
