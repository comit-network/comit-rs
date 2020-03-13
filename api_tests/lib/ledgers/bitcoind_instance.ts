import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { BitcoinNodeConfig, LedgerInstance } from "./ledger_runner";
import { LogReader } from "./log_reader";
import * as path from "path";
import { openAsync, mkdirAsync, writeFileAsync } from "../utils";
import getPort from "get-port";
import BitcoinRpcClient from "bitcoin-core";

export class BitcoindInstance implements LedgerInstance {
    private process: ChildProcess;
    private dataDir: string;
    private username: string;
    private password: string;

    public static async start(projectRoot: string, logDir: string) {
        const instance = new BitcoindInstance(
            projectRoot,
            logDir,
            await getPort({ port: 18444 }),
            await getPort({ port: 18443 }),
            await getPort({ port: 28332 }),
            await getPort({ port: 28333 })
        );

        await instance.start();

        const client = new BitcoinRpcClient({
            network: "regtest",
            host: "localhost",
            port: instance.rpcPort,
            username: instance.username,
            password: instance.password,
        });

        await client.generateToAddress(101, await client.getNewAddress());

        setInterval(async () => {
            await client.generateToAddress(1, await client.getNewAddress());
        }, 1000);

        return instance;
    }

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string,
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
            console.log(`bitcoind exited with ${code || `signal ${signal}`}`);
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
        };
    }

    public stop() {
        this.process.kill("SIGINT");
    }

    private logPath() {
        return path.join(this.dataDir, "bitcoind.log");
    }

    public getDataDir() {
        return this.dataDir;
    }

    public getUsernamePassword() {
        return { username: this.username, password: this.password };
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
