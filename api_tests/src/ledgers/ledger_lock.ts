import { lock } from "proper-lockfile";
import * as path from "path";

/**
 * Locks the given directory for exclusive access.
 *
 * This function provides default parameters to the retries configuration that are suitable for
 * waiting for ledgers to start up. We have to be fairly generous with these timeouts to prevent
 * the lock from failing to quickly if it cannot be acquired.
 */
export default async function ledgerLock(
    lockDir: string
): Promise<() => Promise<void>> {
    return lock(lockDir, {
        lockfilePath: path.join(lockDir, "lock"),
        retries: {
            retries: 10,
            minTimeout: 200,
            maxTimeout: 8000,
        },
    });
}
