import * as bitcoin from "./bitcoin";
import { BitcoindInstance } from "./bitcoind_instance";
import { ParityInstance } from "./parity_instance";
import { EthereumWallet } from "../wallets/ethereum";
import { HarnessGlobal } from "../utils";
import { LndInstance } from "./lnd_instance";
import * as path from "path";
import { EthereumNodeConfig } from "./ethereum";
import { BitcoinNodeConfig } from "./bitcoin";

export interface LedgerConfig {
    bitcoin?: BitcoinNodeConfig;
    ethereum?: EthereumNodeConfig;
    lndAlice?: LndInstance;
    lndBob?: LndInstance;
}

export interface LedgerInstance {
    stop(): void;
}

export class LedgerRunner {
    public readonly runningLedgers: { [key: string]: LedgerInstance };
    private readonly blockTimers: { [key: string]: NodeJS.Timeout };

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string,
        private harnessGlobal: HarnessGlobal
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
                    startedContainers.push({
                        ledger,
                        instance: await BitcoindInstance.start(
                            this.projectRoot,
                            this.logDir
                        ),
                    });
                    break;
                }
                case "ethereum": {
                    startedContainers.push({
                        ledger,
                        instance: await ParityInstance.start(
                            this.projectRoot,
                            this.logDir
                        ),
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
                case "bitcoin": {
                    if (this.harnessGlobal.verbose) {
                        console.log(
                            "Bitcoin: initialization after ledger is running."
                        );
                    }
                    bitcoin.init(await this.getBitcoinClientConfig());
                    await bitcoin.ensureFunding();
                    this.blockTimers.bitcoin = this.harnessGlobal.setInterval(
                        async () => {
                            await bitcoin.generate();
                        },
                        1000
                    );
                    ledgerConfig.bitcoin = await this.getBitcoinClientConfig().catch(
                        () => undefined
                    );
                    break;
                }

                case "ethereum": {
                    const ethereumNodeUrl = await this.getEthereumNodeUrl().catch(
                        () => undefined
                    );
                    const erc20Wallet = new EthereumWallet(ethereumNodeUrl);
                    ledgerConfig.ethereum = {
                        rpc_url: ethereumNodeUrl,
                        tokenContract: await erc20Wallet.deployErc20TokenContract(
                            this.projectRoot
                        ),
                    };
                    if (this.harnessGlobal.verbose) {
                        console.log(
                            "Ethereum: deployed Erc20 contract at %s",
                            ledgerConfig.ethereum.tokenContract
                        );
                    }
                    break;
                }

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

    private async getBitcoinClientConfig(): Promise<BitcoinNodeConfig> {
        const instance = this.runningLedgers.bitcoin as BitcoindInstance;

        if (instance) {
            const { username, password } = instance.getUsernamePassword();

            return {
                network: "regtest",
                host: "localhost",
                rpcPort: instance.rpcPort,
                p2pPort: instance.p2pPort,
                username,
                password,
                dataDir: instance.getDataDir(),
            };
        } else {
            return Promise.reject("bitcoin not yet started");
        }
    }

    private async getEthereumNodeUrl(): Promise<string> {
        const instance = this.runningLedgers.ethereum as ParityInstance;

        if (instance) {
            return `http://localhost:${instance.rpcPort}`;
        } else {
            return Promise.reject("ethereum not yet started");
        }
    }
}
