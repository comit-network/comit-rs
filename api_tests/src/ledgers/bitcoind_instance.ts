import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import * as path from "path";
import { writeFileAsync } from "../utils";
import getPort from "get-port";
import { BitcoinInstance, BitcoinNodeConfig } from "./bitcoin";
import { Logger } from "log4js";
import waitForLogMessage from "../wait_for_log_message";

export class BitcoindInstance implements BitcoinInstance {
    private process: ChildProcess;
    private username: string;
    private password: string;

    public static async new(
        projectRoot: string,
        dataDir: string,
        logger: Logger
    ): Promise<BitcoindInstance> {
        return new BitcoindInstance(
            projectRoot,
            dataDir,
            logger,
            await getPort({ port: 18444 }),
            await getPort({ port: 18443 }),
            await getPort({ port: 28332 }),
            await getPort({ port: 28333 })
        );
    }

    constructor(
        private readonly projectRoot: string,
        private readonly dataDir: string,
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

        await this.createConfigFile(this.dataDir);

        this.process = spawn(bin, [`-datadir=${this.dataDir}`], {
            cwd: this.projectRoot,
            stdio: "ignore",
        });

        this.process.on("exit", (code: number, signal: number) => {
            this.logger.info(
                "bitcoind exited with code",
                code,
                "after signal",
                signal
            );
        });

        await waitForLogMessage(this.logPath(), "init message: Done loading");

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
        return path.join(this.dataDir, "regtest", "debug.log");
    }

    public getDataDir() {
        return this.dataDir;
    }

    private async createConfigFile(dataDir: string) {
        const output = `regtest=1
server=1
printtoconsole=1
rpcallowip=0.0.0.0/0
nodebug=1
rest=1
acceptnonstdtxn=0
zmqpubrawblock=tcp://127.0.0.1:${this.zmqPubRawBlockPort}
zmqpubrawtx=tcp://127.0.0.1:${this.zmqPubRawTxPort}

[regtest]
bind=0.0.0.0:${this.p2pPort}
rpcbind=0.0.0.0:${this.rpcPort}
`;
        const config = path.join(dataDir, "bitcoin.conf");
        await writeFileAsync(config, output);
    }
}
