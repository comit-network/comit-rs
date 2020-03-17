/**
 * Defines an instance of a ledger
 */
export default interface LedgerInstance {
    /**
     * Stop the underlying ledger.
     *
     * This method must never fail.
     */
    stop(): Promise<void>;
}
