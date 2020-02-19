import getPort from "get-port";
import * as bitcoin from "./bitcoin";
import { BitcoinNodeConfig } from "./bitcoin";
import { BitcoindInstance } from "./bitcoind_instance";
import { EthereumNodeConfig } from "./ethereum";
import { ParityInstance } from "./parity_instance";
import { HarnessGlobal } from "./util";
import { EthereumWallet } from "../lib_sdk/wallets/ethereum";

declare var global: HarnessGlobal;

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
                if (global.verbose) {
                    console.log(
                        "Bitcoin: initialization after ledger is running."
                    );
                }
                bitcoin.init(await this.getBitcoinClientConfig());
                await bitcoin.ensureFunding();
                this.blockTimers.bitcoin = global.setInterval(async () => {
                    await bitcoin.generate();
                }, 1000);
            }

            if (ledger === "ethereum") {
                const ethereumConfig = await this.getEthereumNodeConfig();
                const erc20Wallet = new EthereumWallet(ethereumConfig);
                global.tokenContract = await erc20Wallet.deployErc20TokenContract(
                    global.projectRoot
                );
                if (global.verbose) {
                    console.log(
                        "Ethereum: deployed Erc20 contract at %s",
                        global.tokenContract
                    );
                }
            }
        }
    }

    public async stopLedgers() {
        const ledgers = Object.entries(this.runningLedgers);

        const promises = ledgers.map(async ([ledger, ledgerInstance]) => {
            console.log(`Stopping ledger ${ledger}`);

            clearInterval(this.blockTimers[ledger]);
            await ledgerInstance.stop();
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

    private async getEthereumNodeConfig(): Promise<EthereumNodeConfig> {
        const instance = this.runningLedgers.ethereum as ParityInstance;

        if (instance) {
            return {
                rpc_url: `http://localhost:${instance.rpcPort}`,
            };
        } else {
            return Promise.reject("ethereum not yet started");
        }
    }
}
