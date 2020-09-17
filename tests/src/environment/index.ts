import { Network } from "../wallets/bitcoin";
import { Global } from "@jest/types";
import { CndConfig } from "./cnd_config";
import { Logger } from "log4js";
import { LndClient } from "../wallets/lightning";

export interface HarnessGlobal extends Global.Global {
    environment: Environment;
    lndClients: {
        alice?: LndClient;
        bob?: LndClient;
    };
    tokenContract: string;
    cargoTargetDir: string;
    cndConfigOverrides: Partial<CndConfig>;

    getDataDir: (program: string) => Promise<string>;
    getLogFile: (pathElements: string[]) => string;
    getLogger: (categories: string[]) => Logger;
}

export interface BitcoinNode {
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

export interface LightningNode {
    p2pSocket: string;
    grpcSocket: string;
    tlsCertPath: string;
    macaroonPath: string;
    restPort: number;
    dataDir: string;
}

export interface EthereumNode {
    devAccount: string;
    rpc_url: string;
    tokenContract: string;
    chain_id: number;
}

export interface Environment {
    bitcoin?: BitcoinNode;
    ethereum?: EthereumNode;
    aliceLnd?: LightningNode;
    bobLnd?: LightningNode;
}

export interface Startable {
    start(): Promise<void>;
}
