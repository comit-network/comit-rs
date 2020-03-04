import getPort from "get-port";
import * as bitcoin from "./bitcoin";
import { BitcoinNodeConfig } from "./bitcoin";
import { BitcoindInstance } from "./bitcoind_instance";
import { ParityInstance } from "./parity_instance";
import { EthereumWallet } from "../wallets/ethereum";
import { HarnessGlobal } from "../utils";
import { EthereumNodeConfig } from "./ethereum";

export interface LedgerConfig {
    bitcoin?: BitcoinNodeConfig;
    ethereum?: EthereumNodeConfig;
}

export interface LedgerInstance {
    start(): Promise<LedgerInstance>;
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

        const returnValue: LedgerConfig = {};
        const promises = toBeStarted.map(async ledger => {
            console.log(`Starting ledger ${ledger}`);

            switch (ledger) {
                case "bitcoin": {
                    const instance = new BitcoindInstance(
                        this.projectRoot,
                        this.logDir,
                        await getPort({ port: 18444 }),
                        await getPort({ port: 18443 }),
                        await getPort({ port: 28332 }),
                        await getPort({ port: 28333 })
                    );
                    return {
                        ledger,
                        instance: await instance.start(),
                    };
                }
                case "ethereum": {
                    const instance = new ParityInstance(
                        this.projectRoot,
                        this.logDir,
                        await getPort({ port: 8545 })
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
                returnValue.bitcoin = await this.getBitcoinClientConfig().catch(
                    () => undefined
                );
            }

            if (ledger === "ethereum") {
                const ethereumNodeUrl = await this.getEthereumNodeUrl().catch(
                    () => undefined
                );
                const erc20Wallet = new EthereumWallet(ethereumNodeUrl);
                returnValue.ethereum = {
                    rpc_url: ethereumNodeUrl,
                    tokenContract: await erc20Wallet.deployErc20TokenContract(
                        this.projectRoot
                    ),
                };
                if (this.harnessGlobal.verbose) {
                    console.log(
                        "Ethereum: deployed Erc20 contract at %s",
                        returnValue.ethereum.tokenContract
                    );
                }
            }
        }

        return returnValue;
    }

    public async stopLedgers() {
        const ledgers = Object.entries(this.runningLedgers);

        const promises = ledgers.map(async ([ledger, ledgerInstance]) => {
            console.log(`Stopping ledger ${ledger}`);

            clearInterval(this.blockTimers[ledger]);
            ledgerInstance.stop();
            delete this.runningLedgers[ledger];
        });

        await Promise.all(promises);
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
