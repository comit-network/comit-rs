import { promises as asyncFs } from "fs";
import * as fs from "fs";
import { promisify } from "util";
import { Global } from "@jest/types";
import rimraf from "rimraf";
import { exec } from "child_process";
import { LightningWallet } from "./wallets/lightning";
import { Logger } from "log4js";
import {
    BitcoinNodeConfig,
    EthereumNodeConfig,
    LightningNodeConfig,
} from "./ledgers";

export interface HarnessGlobal extends Global.Global {
    ledgerConfigs: LedgerConfig;
    lndWallets: {
        alice?: LightningWallet;
        bob?: LightningWallet;
    };
    tokenContract: string;
    gethLockDir: string;
    cargoTargetDir: string;

    getDataDir: (program: string) => Promise<string>;
    getLogFile: (pathElements: string[]) => string;
    getLogger: (categories: string[]) => Logger;
}

export interface LedgerConfig {
    bitcoin?: BitcoinNodeConfig;
    ethereum?: EthereumNodeConfig;
    aliceLnd?: LightningNodeConfig;
    bobLnd?: LightningNodeConfig;
}

export const existsAsync = (filepath: string) =>
    asyncFs.access(filepath, fs.constants.F_OK);
export const openAsync = promisify(fs.open);
export const rimrafAsync = promisify(rimraf);
export const execAsync = promisify(exec);

export async function sleep(time: number) {
    return new Promise((res) => {
        setTimeout(res, time);
    });
}

export async function waitUntilFileExists(filepath: string) {
    let logFileExists = false;
    do {
        try {
            await existsAsync(filepath);
            logFileExists = true;
        } catch (e) {
            await sleep(500);
        }
    } while (!logFileExists);
}
