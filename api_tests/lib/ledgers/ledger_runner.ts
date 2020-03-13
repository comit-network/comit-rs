import { BitcoindInstance } from "./bitcoind_instance";
import { ParityInstance } from "./parity_instance";
import { LndInstance } from "./lnd_instance";
import * as path from "path";

export interface LedgerConfig {
    bitcoin?: BitcoinNodeConfig;
    ethereum?: EthereumNodeConfig;
    lndAlice?: LndInstance;
    lndBob?: LndInstance;
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

export interface EthereumNodeConfig {
    rpc_url: string;
    tokenContract: string;
}

export interface LedgerInstance {
    stop(): void;
}

export class LedgerRunner {
    public readonly runningLedgers: { [key: string]: LedgerInstance };
    private readonly blockTimers: { [key: string]: NodeJS.Timeout };

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string
    ) {
        this.runningLedgers = {};
        this.blockTimers = {};
    }

    public async ensureLedgersRunning(
        ledgers: string[]
    ): Promise<LedgerConfig> {
        const toBeStarted = ledgers.filter(name => !this.runningLedgers[name]);

        const startedContainers = [];

        const ledgerConfig: LedgerConfig = {};

        for (const ledger of toBeStarted) {
            console.log(`Starting ledger ${ledger}`);

            switch (ledger) {
                case "bitcoin": {
                    const instance = await BitcoindInstance.start(
                        this.projectRoot,
                        this.logDir
                    );

                    ledgerConfig.bitcoin = instance.config;

                    startedContainers.push({
                        ledger,
                        instance,
                    });
                    break;
                }
                case "ethereum": {
                    const parity = await ParityInstance.start(
                        this.projectRoot,
                        this.logDir
                    );

                    ledgerConfig.ethereum = parity.config;

                    startedContainers.push({
                        ledger,
                        instance: parity,
                    });
                    break;
                }
                case "lnd-alice": {
                    startedContainers.push({
                        ledger,
                        instance: await LndInstance.start(
                            this.logDir,
                            "lnd-alice",
                            path.join(this.logDir, "bitcoind")
                        ),
                    });
                    break;
                }
                case "lnd-bob": {
                    startedContainers.push({
                        ledger,
                        instance: await LndInstance.start(
                            this.logDir,
                            "lnd-bob",
                            path.join(this.logDir, "bitcoind")
                        ),
                    });
                    break;
                }
                default: {
                    throw new Error(`LedgerRunner does not support ${ledger}`);
                }
            }
        }

        for (const { ledger, instance } of startedContainers) {
            this.runningLedgers[ledger] = instance;

            switch (ledger) {
                case "lnd-alice": {
                    ledgerConfig.lndAlice = instance as LndInstance;
                    break;
                }

                case "lnd-bob": {
                    ledgerConfig.lndBob = instance as LndInstance;
                    break;
                }
            }
        }

        return ledgerConfig;
    }

    public stopLedgers() {
        const ledgers = Object.entries(this.runningLedgers);

        ledgers.map(([ledger, ledgerInstance]) => {
            console.log(`Stopping ledger ${ledger}`);
            clearInterval(this.blockTimers[ledger]);
            ledgerInstance.stop();
            delete this.runningLedgers[ledger];
        });
    }
}
