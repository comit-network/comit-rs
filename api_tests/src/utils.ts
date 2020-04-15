import { ethers } from "ethers";
import { Actor } from "./actors/actor";
import { SwapRequest } from "comit-sdk";
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
    parityLockDir: string;
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

export const unlinkAsync = promisify(fs.unlink);
export const existsAsync = promisify(fs.exists);
export const openAsync = promisify(fs.open);
export const mkdirAsync = promisify(fs.mkdir);
export const writeFileAsync = promisify(fs.writeFile);
export const readFileAsync = promisify(fs.readFile);
export const rimrafAsync = promisify(rimraf);
export const execAsync = promisify(exec);

export async function sleep(time: number) {
    return new Promise((res) => {
        setTimeout(res, time);
    });
}

export const DEFAULT_ALPHA = {
    ledger: {
        name: "bitcoin",
        network: "regtest",
    },
    asset: {
        name: "bitcoin",
        quantity: {
            bob: "100000000",
            charlie: "200000000",
            reasonable: "100000000",
            stingy: "100",
        },
    },
    expiry: new Date("2080-06-11T23:00:00Z").getTime() / 1000,
};

const DEFAULT_BETA = {
    ledger: {
        name: "ethereum",
        chain_id: 17,
    },
    asset: {
        name: "ether",
        quantity: {
            bob: ethers.utils.parseEther("10").toString(),
            charlie: ethers.utils.parseEther("20").toString(),
        },
    },
    expiry: new Date("2080-06-11T13:00:00Z").getTime() / 1000,
};
export async function createDefaultSwapRequest(counterParty: Actor) {
    const swapRequest: SwapRequest = {
        alpha_ledger: {
            name: DEFAULT_ALPHA.ledger.name,
            network: DEFAULT_ALPHA.ledger.network,
        },
        beta_ledger: {
            name: DEFAULT_BETA.ledger.name,
            chain_id: DEFAULT_BETA.ledger.chain_id,
        },
        alpha_asset: {
            name: DEFAULT_ALPHA.asset.name,
            quantity: DEFAULT_ALPHA.asset.quantity.bob,
        },
        beta_asset: {
            name: DEFAULT_BETA.asset.name,
            quantity: DEFAULT_BETA.asset.quantity.bob,
        },
        beta_ledger_redeem_identity:
            "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        alpha_expiry: DEFAULT_ALPHA.expiry,
        beta_expiry: DEFAULT_BETA.expiry,
        peer: {
            peer_id: await counterParty.cnd.getPeerId(),
            address_hint: await counterParty.cnd
                .getPeerListenAddresses()
                .then((addresses) => addresses[0]),
        },
    };
    return swapRequest;
}

export async function waitUntilFileExists(filepath: string) {
    let logFileExists = false;
    do {
        await sleep(500);
        logFileExists = await existsAsync(filepath);
    } while (!logFileExists);
}
