import { Global } from "@jest/types";
import { CndConfig } from "./cnd_config";
import { Logger } from "log4js";

export interface HarnessGlobal extends Global.Global {
    environment: Environment;
    tokenContract: string;
    cargoTargetDir: string;
    cndConfigOverrides: Partial<CndConfig>;

    getDataDir: (program: string) => Promise<string>;
    getLogFile: (pathElements: string[]) => string;
    getLogger: (categories: string[]) => Logger;
}

export interface BitcoinNode {
    network: "regtest";
    username: string;
    password: string;
    host: string;
    rpcPort: number;
    rpcUrl: string;
    p2pPort: number;
    dataDir: string;
    minerWallet?: string;
}

export interface EthereumNode {
    devAccount: string;
    rpc_url: string;
    tokenContract: string;
    chain_id: number;
}

export interface FakeTreasuryService {
    host: string;
}

export interface Environment {
    bitcoin?: BitcoinNode;
    ethereum?: EthereumNode;
    treasury?: FakeTreasuryService;
}

export interface Startable {
    start(): Promise<void>;
}
