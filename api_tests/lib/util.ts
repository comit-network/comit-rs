import { TestConfig } from "./actor";
import { BtcConfig } from "./bitcoin";
import { EthConfig } from "./ethereum";

let _test_rng_counter = 0;

export function test_rng() {
    _test_rng_counter++;
    return Buffer.from(("" + _test_rng_counter).padStart(32, "0"));
}

export async function sleep(time: number) {
    return new Promise((res, rej) => {
        setTimeout(res, time);
    });
}

// Should go away with consolidation #767
import Global = NodeJS.Global;

export interface HarnessGlobal extends Global {
    config: TestConfig;
    ledgers_config: {
        bitcoin: BtcConfig;
        ethereum: EthConfig;
    };
    test_root: string;
    project_root: string;
}
