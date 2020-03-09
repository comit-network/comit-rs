import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { LedgerInstance } from "./ledger_runner";
import { LogReader } from "./log_reader";
import * as path from "path";
import { openAsync, mkdirAsync, writeFileAsync } from "../utils";

export class BitcoindInstance implements LedgerInstance {
    private process: ChildProcess;
    private dataDir: string;
    private username: string;
    private password: string;

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
        await logReader.waitForLogMessage("Wallet completed loading");

        const result = fs.readFileSync(
            path.join(this.dataDir, "regtest", ".cookie"),
            "utf8"
        );
        const [username, password] = result.split(":");

        this.username = username;
        this.password = password;

        return this;
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
