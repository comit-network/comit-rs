import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tmp from "tmp";
import { promisify } from "util";
import { LogReader } from "./log_reader";

const openAsync = promisify(fs.open);

export class BitcoindInstance {
    private process: ChildProcess;
    private dbDir: any;
    private username: string;
    private password: string;

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string,
        private readonly p2pPort: number,
        private readonly rpcPort: number
    ) {}

    public async start() {
        const bin = process.env.BITCOIND_BIN
            ? process.env.BITCOIND_BIN
            : this.projectRoot +
              "/blockchain_nodes/bitcoin/bitcoin-0.17.0/bin/bitcoind";

        this.dbDir = tmp.dirSync();

        this.process = spawn(
            bin,
            [
                `-datadir=${this.dbDir.name}`,
                "-regtest",
                "-server",
                "-printtoconsole",
                `-bind=0.0.0.0:${this.p2pPort}`,
                `-rpcbind=0.0.0.0:${this.rpcPort}`,
                "-rpcallowip=0.0.0.0/0",
                "-nodebug",
                "-acceptnonstdtxn=0",
                "-rest",
            ],
            {
                cwd: this.projectRoot,
                stdio: [
                    "ignore", // stdin
                    await openAsync(this.logDir + "/bitcoind.log", "w"), // stdout
                    await openAsync(this.logDir + "/bitcoind.log", "w"), // stderr
                ],
            }
        );

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
}
