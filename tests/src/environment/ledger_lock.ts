import { lock } from "proper-lockfile";
import * as path from "path";
import { promises as asyncFc } from "fs";

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
    const lockFile = path.join(lockDir, "ledger.lock");

    await asyncFc.mkdir(lockFile, {
        recursive: true,
    });
    return lock(lockFile, {
        retries: {
            retries: 60,
            factor: 1,
            minTimeout: 500,
        },
    });
}
