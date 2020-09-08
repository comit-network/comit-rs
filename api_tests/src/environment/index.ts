import { Network } from "../wallets/bitcoin";
import { Global } from "@jest/types";
import { LightningWallet } from "../wallets/lightning";
import { CndConfigFile } from "../config";
import { Logger } from "log4js";

export interface HarnessGlobal extends Global.Global {
    ledgerConfigs: LedgerConfig;
    lndWallets: {
        alice?: LightningWallet;
        bob?: LightningWallet;
    };
    tokenContract: string;
    cargoTargetDir: string;
    cndConfigOverrides: Partial<CndConfigFile>;

    getDataDir: (program: string) => Promise<string>;
    getLogFile: (pathElements: string[]) => string;
    getLogger: (categories: string[]) => Logger;
}

export interface BitcoinNodeConfig {
    network: Network;
    username: string;
    password: string;
    host: string;
    rpcPort: number;
    rpcUrl: string;
    p2pPort: number;
    dataDir: string;
    minerWallet?: string;
}

export interface LightningNodeConfig {
    p2pSocket: string;
    grpcSocket: string;
    tlsCertPath: string;
    macaroonPath: string;
    restPort: number;
    dataDir: string;
}

export interface EthereumNodeConfig {
    devAccount: string;
    rpc_url: string;
    tokenContract: string;
    chain_id: number;
}

export interface LedgerConfig {
    bitcoin?: BitcoinNodeConfig;
    ethereum?: EthereumNodeConfig;
    aliceLnd?: LightningNodeConfig;
    bobLnd?: LightningNodeConfig;
}

export interface LedgerInstance {
    start(): Promise<void>;
}
