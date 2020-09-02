import { Network } from "../wallets/bitcoin";

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
    rpc_url: string;
    tokenContract: string;
    dev_account_key: string;
    chain_id: number;
}

export interface LedgerInstance {
    start(): Promise<void>;
}
