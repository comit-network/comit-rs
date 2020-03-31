import { Lnd } from "comit-sdk";
import LedgerInstance from "./ledger_instance";

/**
 * An instance of the Lightning ledger for use in the e2e tests.
 *
 * This class is compatible with anything that implements {@link LightningInstance}.
 *
 * Compared to {@link BitcoinLedger} and {@link EthereumLedger}, there is nothing
 * to be done after a {@link LightningInstance} is started. If this ever changes,
 * this class is the place where to put this information.
 */
export default class LightningLedger implements LedgerInstance {
    public static async start(instance: LightningInstance) {
        await instance.start();

        return new LightningLedger(instance);
    }

    constructor(private readonly instance: LightningInstance) {}

    async stop(): Promise<void> {
        return this.instance.stop();
    }

    get config(): LightningNodeConfig {
        return this.instance.config;
    }
}

export interface LightningInstance {
    config: LightningNodeConfig;

    start(): Promise<void>;
    stop(): Promise<void>;
}

export interface LightningNodeConfig {
    p2pSocket: string;
    // sucks that we have to leak here that the instance is LND under the hood but we can't do much about that :)
    lnd: Lnd;
    restPort: number;
    dataDir: string;
}
