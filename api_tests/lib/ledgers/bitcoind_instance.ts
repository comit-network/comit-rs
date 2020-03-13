import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { LogReader } from "./log_reader";
import * as path from "path";
import { mkdirAsync, openAsync, writeFileAsync } from "../utils";
import getPort from "get-port";
import { BitcoinInstance, BitcoinNodeConfig } from "./bitcoin";
import { Logger } from "log4js";

export class BitcoindInstance implements BitcoinInstance {
    private process: ChildProcess;
    private dataDir: string;
    private username: string;
    private password: string;

    public static async new(
        projectRoot: string,
        logDir: string,
        logger: Logger
    ): Promise<BitcoindInstance> {
        return new BitcoindInstance(
            projectRoot,
            logDir,
            logger,
            await getPort({ port: 18444 }),
            await getPort({ port: 18443 }),
            await getPort({ port: 28332 }),
            await getPort({ port: 28333 })
        );
    }

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string,
        private readonly logger: Logger,
        public readonly p2pPort: number,
        public readonly rpcPort: number,
        public readonly zmqPubRawBlockPort: number,
        public readonly zmqPubRawTxPort: number
    ) {}

    public async start() {
        const bin = process.env.BITCOIND_BIN
            ? process.env.BITCOIND_BIN
            : path.join(
                  this.projectRoot,
                  "blockchain_nodes",
                  "bitcoin",
                  "bitcoin-0.17.0",
                  "bin",
                  "bitcoind"
              );
        this.logger.info("Using binary", bin);

        this.dataDir = path.join(this.logDir, "bitcoind");
        await mkdirAsync(this.dataDir, "755");
        await this.createConfigFile(this.dataDir);

        const log = this.logPath();
        this.process = spawn(bin, [`-datadir=${this.dataDir}`], {
            cwd: this.projectRoot,
            stdio: [
                "ignore", // stdin
                await openAsync(log, "w"), // stdout
                await openAsync(log, "w"), // stderr
            ],
        });

        this.process.on("exit", (code: number, signal: number) => {
            this.logger.info(
                "binary exited with code",
                code,
                "after signal",
                signal
            );
        });

        const logReader = new LogReader(this.logPath());
        await logReader.waitForLogMessage("init message: Done loading");

        const result = fs.readFileSync(
            path.join(this.dataDir, "regtest", ".cookie"),
            "utf8"
        );
        const [username, password] = result.split(":");

        this.username = username;
        this.password = password;

        this.logger.info("bitcoind started with PID", this.process.pid);
    }

    public get config(): BitcoinNodeConfig {
        return {
            network: "regtest",
            host: "localhost",
            rpcPort: this.rpcPort,
            p2pPort: this.p2pPort,
            username: this.username,
            password: this.password,
            dataDir: this.getDataDir(),
            rpcUrl: `http://localhost:${this.rpcPort}`,
        };
    }

    public async stop() {
        this.logger.info("Stopping bitcoind instance");

        this.process.kill("SIGINT");
    }

    private logPath() {
        return path.join(this.dataDir, "bitcoind.log");
    }

    public getDataDir() {
        return this.dataDir;
    }

    private async createConfigFile(dataDir: string) {
        const output = `regtest=1
server=1
printtoconsole=1
bind=0.0.0.0:${this.p2pPort}
rpcbind=0.0.0.0:${this.rpcPort}
rpcallowip=0.0.0.0/0
nodebug=1
rest=1
acceptnonstdtxn=0
zmqpubrawblock=tcp://127.0.0.1:${this.zmqPubRawBlockPort}
zmqpubrawtx=tcp://127.0.0.1:${this.zmqPubRawTxPort}
`;
        const config = path.join(dataDir, "bitcoin.conf");
        await writeFileAsync(config, output);
    }
}
