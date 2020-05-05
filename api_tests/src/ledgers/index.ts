export interface BitcoinNodeConfig {
    network: string;
    username: string;
    password: string;
    host: string;
    rpcPort: number;
    rpcUrl: string;
    p2pPort: number;
    dataDir: string;
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
}

export interface LedgerInstance {
    start(): Promise<void>;
}
