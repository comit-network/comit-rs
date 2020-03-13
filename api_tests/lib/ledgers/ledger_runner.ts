import { BitcoindInstance } from "./bitcoind_instance";
import { ParityInstance } from "./parity_instance";
import { LndInstance } from "./lnd_instance";
import * as path from "path";
import BitcoinLedger, { BitcoinNodeConfig } from "./bitcoin";
import EthereumLedger, { EthereumNodeConfig } from "./ethereum";
import LightningLedger, { LightningNodeConfig } from "./lightning";

export interface LedgerConfig {
    bitcoin?: BitcoinNodeConfig;
    ethereum?: EthereumNodeConfig;
    lndAlice?: LightningNodeConfig;
    lndBob?: LightningNodeConfig;
}

export interface LedgerInstance {
    stop(): Promise<void>;
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
                    const bitcoin = await BitcoinLedger.start(
                        await BitcoindInstance.new(
                            this.projectRoot,
                            this.logDir
                        )
                    );

                    ledgerConfig.bitcoin = bitcoin.config;

                    startedContainers.push({
                        ledger,
                        instance: bitcoin,
                    });
                    break;
                }
                case "ethereum": {
                    const ethereum = await EthereumLedger.start(
                        await ParityInstance.new(this.projectRoot, this.logDir)
                    );

                    ledgerConfig.ethereum = ethereum.config;

                    startedContainers.push({
                        ledger,
                        instance: ethereum,
                    });
                    break;
                }
                case "lnd-alice": {
                    const lightning = await LightningLedger.start(
                        await LndInstance.new(
                            this.logDir,
                            "lnd-alice",
                            path.join(this.logDir, "bitcoind")
                        )
                    );

                    ledgerConfig.lndAlice = lightning.config;

                    startedContainers.push({
                        ledger,
                        instance: lightning,
                    });

                    break;
                }
                case "lnd-bob": {
                    const lightning = await LightningLedger.start(
                        await LndInstance.new(
                            this.logDir,
                            "lnd-bob",
                            path.join(this.logDir, "bitcoind")
                        )
                    );
                    startedContainers.push({
                        ledger,
                        instance: lightning,
                    });

                    ledgerConfig.lndBob = lightning.config;

                    break;
                }
                default: {
                    throw new Error(`LedgerRunner does not support ${ledger}`);
                }
            }
        }

        return ledgerConfig;
    }

    public async stopLedgers() {
        const ledgers = Object.entries(this.runningLedgers);

        for (const [ledger, instance] of ledgers) {
            console.log(`Stopping ledger ${ledger}`);
            clearInterval(this.blockTimers[ledger]);
            await instance.stop();
            delete this.runningLedgers[ledger];
        }
    }
}
