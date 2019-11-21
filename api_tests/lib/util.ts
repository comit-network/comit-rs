import { LedgerConfig } from "./ledger_runner";

let testRngCounter = 0;

export function test_rng() {
    testRngCounter++;
    return Buffer.from(("" + testRngCounter).padStart(32, "0"));
}

export async function sleep(time: number) {
    return new Promise(res => {
        setTimeout(res, time);
    });
}

export function seconds_until(time: number): number {
    const diff = time - Math.floor(Date.now() / 1000);

    if (diff > 0) {
        return diff;
    } else {
        return 0;
    }
}

/// This is needed to use the global variable in TypeScript
import Global = NodeJS.Global;

export interface HarnessGlobal extends Global {
    ledgerConfigs: LedgerConfig;
    testRoot: string;
    projectRoot: string;
    logRoot: string;
}
