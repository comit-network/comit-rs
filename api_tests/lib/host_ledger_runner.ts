import * as bitcoin from "./bitcoin";
import { BitcoinNodeConfig } from "./bitcoin";
import { BitcoindInstance } from "./bitcoind_instance";
import { EthereumNodeConfig } from "./ethereum";
import { ParityInstance } from "./parity_instance";

export interface LedgerConfig {
    bitcoin?: BitcoinNodeConfig;
    ethereum?: EthereumNodeConfig;
}

export class LedgerRunner {
    public readonly runningLedgers: { [key: string]: any };
    private readonly blockTimers: { [key: string]: NodeJS.Timeout };

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string
    ) {
        this.runningLedgers = {};
        this.blockTimers = {};
    }

    public async ensureLedgersRunning(ledgers: string[]) {
        const toBeStarted = ledgers.filter(name => !this.runningLedgers[name]);

        const promises = toBeStarted.map(async ledger => {
            console.log(`Starting ledger ${ledger}`);

            switch (ledger) {
                case "bitcoin": {
                    const instance = new BitcoindInstance(
                        this.projectRoot,
                        this.logDir
                    );
                    return {
                        ledger,
                        instance: await instance.start(),
                    };
                }
                case "ethereum": {
                    const instance = new ParityInstance(
                        this.projectRoot,
                        this.logDir
                    );
                    return {
                        ledger,
                        instance: await instance.start(),
                    };
                }
                default: {
                    throw new Error(`Ledgerrunner does not support ${ledger}`);
                }
            }
        });

        const startedContainers = await Promise.all(promises);

        for (const { ledger, instance } of startedContainers) {
            this.runningLedgers[ledger] = instance;

            if (ledger === "bitcoin") {
                bitcoin.init(await this.getBitcoinClientConfig());
                this.blockTimers.bitcoin = global.setInterval(async () => {
                    await bitcoin.generate();
                }, 1000);
            }
        }
    }

    public async stopLedgers() {
        const ledgers = Object.entries(this.runningLedgers);

        const promises = ledgers.map(async ([ledger, container]) => {
            console.log(`Stopping ledger ${ledger}`);

            clearInterval(this.blockTimers[ledger]);
            await container.stop();
            delete this.runningLedgers[ledger];
        });

        await Promise.all(promises);
    }

    public async getLedgerConfig(): Promise<LedgerConfig> {
        return {
            bitcoin: await this.getBitcoinClientConfig().catch(() => undefined),
            ethereum: await this.getEthereumNodeConfig().catch(() => undefined),
        };
    }

    private async getBitcoinClientConfig(): Promise<BitcoinNodeConfig> {
        const container = this.runningLedgers.bitcoin;

        if (container) {
            const { username, password } = container.getUsernamePassword();

            return {
                network: "regtest",
                host: "localhost",
                rpcPort: 18443,
                p2pPort: 18444,
                username,
                password,
            };
        } else {
            return Promise.reject("bitcoin not yet started");
        }
    }

    private async getEthereumNodeConfig(): Promise<EthereumNodeConfig> {
        const container = this.runningLedgers.ethereum;

        if (container) {
            const host = "localhost";
            const port = 8545;

            return {
                rpc_url: `http://${host}:${port}`,
            };
        } else {
            return Promise.reject("ethereum not yet started");
        }
    }
}
