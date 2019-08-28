import { createWriteStream } from "fs";
import { GenericContainer } from "testcontainers";
import { StartedTestContainer } from "testcontainers/dist/test-container";
import { LogWaitStrategy } from "testcontainers/dist/wait-strategy";
import * as bitcoin from "./bitcoin";
import { BitcoinNodeConfig } from "./bitcoin";
import { EthereumNodeConfig } from "./ethereum";

export interface LedgerConfig {
    bitcoin?: BitcoinNodeConfig;
    ethereum?: EthereumNodeConfig;
}

export class LedgerRunner {
    public readonly runningLedgers: { [key: string]: StartedTestContainer };
    private readonly blockTimers: { [key: string]: NodeJS.Timeout };

    constructor(private readonly logDir: string) {
        this.runningLedgers = {};
        this.blockTimers = {};
    }

    public async ensureLedgersRunning(ledgers: string[]) {
        const toBeStarted = ledgers.filter(name => !this.runningLedgers[name]);

        const promises = toBeStarted.map(async ledger => {
            console.log(`Starting ledger ${ledger}`);

            switch (ledger) {
                case "bitcoin": {
                    return {
                        ledger,
                        container: await startBitcoinContainer(),
                    };
                }
                case "ethereum": {
                    return {
                        ledger,
                        container: await startEthereumContainer(),
                    };
                }
                default: {
                    throw new Error(`Ledgerrunner does not support ${ledger}`);
                }
            }
        });

        const startedContainers = await Promise.all(promises);

        for (const { ledger, container } of startedContainers) {
            this.runningLedgers[ledger] = container;

            // @ts-ignore hack around the fact that container is not publicly exposed
            const containerLogs = await container.container.logs();

            const logFile = createWriteStream(`${this.logDir}/${ledger}.log`, {
                encoding: "utf8",
            });

            containerLogs.on("data", (buffer: Buffer) => {
                buffer = sanitizeBuffer(buffer);

                logFile.write(buffer);

                if (buffer.length > 0) {
                    logFile.write("\n");
                }
            });

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
            const result = await container.exec([
                "cat",
                "/root/.bitcoin/regtest/.cookie",
            ]);
            const [, password] = result.output.split(":");

            return {
                host: container.getContainerIpAddress(),
                rpcPort: container.getMappedPort(18443),
                zmqPort: container.getMappedPort(28332),
                username: "__cookie__",
                password,
            };
        } else {
            return Promise.reject("bitcoin not yet started");
        }
    }

    private async getEthereumNodeConfig(): Promise<EthereumNodeConfig> {
        const container = this.runningLedgers.ethereum;

        if (container) {
            const host = container.getContainerIpAddress();
            const port = container.getMappedPort(8545);

            return {
                rpc_url: `http://${host}:${port}`,
            };
        } else {
            return Promise.reject("ethereum not yet started");
        }
    }
}

/*
 * For some weird reason, the log buffer contains weird prefixes
 *
 * This function removes those prefixes so that the logs can be printed to the file.
 */
function sanitizeBuffer(buffer: Buffer) {
    if (buffer.indexOf(Buffer.from("01000000000000", "hex")) === 0) {
        buffer = buffer.slice(8);
    }
    if (buffer.indexOf(Buffer.from("020000000000", "hex")) === 0) {
        buffer = buffer.slice(8);
    }
    if (buffer.indexOf(Buffer.from("bfbd", "hex")) === 0) {
        buffer = buffer.slice(2);
    }
    return buffer;
}

async function startBitcoinContainer(): Promise<StartedTestContainer> {
    return new GenericContainer("coblox/bitcoin-core", "0.17.0")
        .withCmd([
            "-regtest",
            "-server",
            "-printtoconsole",
            "-rpcbind=0.0.0.0:18443",
            "-rpcallowip=0.0.0.0/0",
            "-debug=1",
            "-zmqpubrawblock=tcp://*:28332",
            "-zmqpubrawtx=tcp://*:28333",
            "-acceptnonstdtxn=0",
        ])
        .withExposedPorts(18443, 28332)
        .withWaitStrategy(new LogWaitStrategy("Flushed wallet.dat"))
        .start();
}

async function startEthereumContainer(): Promise<StartedTestContainer> {
    return new GenericContainer("parity/parity", "v2.5.0")
        .withCmd([
            "--config=dev",
            "--jsonrpc-apis=all",
            "--unsafe-expose",
            "--tracing=on",
            "--logging=debug,ethcore-miner=trace,miner=trace,rpc=trace,tokio_core=warn,tokio_reactor=warn",
            "--jsonrpc-cors='all'",
        ])
        .withExposedPorts(8545)
        .withWaitStrategy(new LogWaitStrategy("Public node URL:"))
        .start();
}
