import { LedgerInstance } from "./ledger_runner";
import BitcoinRpcClient from "bitcoin-core";

/**
 * An instance of the Bitcoin ledger for use in the e2e tests.
 *
 * This class is compatible with anything that implements {@link BitcoinInstance}.
 *
 * For the e2e tests to work properly, we need to continuously mine bitcoin blocks.
 * This class takes care of spawning a miner after the Bitcoin blockchain has
 * been setup, regardless of how that is achieved (Docker container, bitcoind instance, etc).
 */
export default class BitcoinLedger implements LedgerInstance {
    public static async start(instance: BitcoinInstance) {
        await instance.start();

        const { rpcPort, username, password } = instance.config;

        const client = new BitcoinRpcClient({
            network: "regtest",
            host: "localhost",
            port: rpcPort,
            username,
            password,
        });

        await client.generateToAddress(101, await client.getNewAddress());

        setInterval(async () => {
            await client.generateToAddress(1, await client.getNewAddress());
        }, 1000);

        return new BitcoinLedger(instance);
    }

    constructor(private readonly instance: BitcoinInstance) {}

    public async stop(): Promise<void> {
        await this.instance.stop();
    }

    public get config(): BitcoinNodeConfig {
        return this.instance.config;
    }
}

export interface BitcoinInstance {
    config: BitcoinNodeConfig;

    start(): Promise<void>;
    stop(): Promise<void>;
}

export interface BitcoinNodeConfig {
    network: string;
    username: string;
    password: string;
    host: string;
    rpcPort: number;
    p2pPort: number;
    dataDir: string;
}
