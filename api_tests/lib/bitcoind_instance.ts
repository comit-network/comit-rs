import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tmp from "tmp";
import { promisify } from "util";
import { LedgerInstance } from "./ledger_runner";
import { LogReader } from "./log_reader";

const openAsync = promisify(fs.open);

export class BitcoindInstance implements LedgerInstance {
    private process: ChildProcess;
    private dbDir: any;
    private username: string;
    private password: string;

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string,
        public readonly p2pPort: number,
        public readonly rpcPort: number
    ) {
        this.dbDir = tmp.dirSync();
        this.writeLogFile();
    }

    public async start() {
        const bin = process.env.BITCOIND_BIN
            ? process.env.BITCOIND_BIN
            : this.projectRoot +
              "/blockchain_nodes/bitcoin/bitcoin-0.17.0/bin/bitcoind";

        this.process = spawn(bin, [`-datadir=${this.dbDir.name}`], {
            cwd: this.projectRoot,
            stdio: [
                "ignore", // stdin
                await openAsync(this.logDir + "/bitcoind.log", "w"), // stdout
                await openAsync(this.logDir + "/bitcoind.log", "w"), // stderr
            ],
        });

        this.process.on("exit", (code: number, signal: number) => {
            console.log(`bitcoind exited with ${code || "signal " + signal}`);
        });

        const logReader = new LogReader(this.logDir + "/bitcoind.log");
        await logReader.waitForLogMessage("Wallet completed loading");

        const result = fs.readFileSync(
            `${this.dbDir.name}/regtest/.cookie`,
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

    public getUsernamePassword() {
        return { username: this.username, password: this.password };
    }

    private writeLogFile() {
        const output = `regtest=1
server=1
printtoconsole=1
bind=0.0.0.0:${this.p2pPort}
rpcbind=0.0.0.0:${this.rpcPort}
rpcallowip=0.0.0.0/0
nodebug=1
rest=1
acceptnonstdtxn=0
zmqpubrawblock=tcp://127.0.0.1:28332
zmqpubrawtx=tcp://127.0.0.1:28333
`;
        const config = this.dbDir.name + "/bitcoin.conf";

        fs.writeFile(config, output, function(err: any) {
            if (err) {
                return console.error(err);
            }
            console.log("bitcoind config file created: %s", config);
        });
    }
}
