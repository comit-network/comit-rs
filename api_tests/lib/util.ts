import { TestConfig } from "./actor";
import { BitcoinNodeConfig } from "./bitcoin";
import { EthereumNodeConfig } from "./ethereum";

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
    config: TestConfig;
    ledgers_config: {
        bitcoin: BitcoinNodeConfig;
        ethereum: EthereumNodeConfig;
    };
    test_root: string;
    project_root: string;
}
