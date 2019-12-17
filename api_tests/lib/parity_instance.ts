import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tmp from "tmp";
import { promisify } from "util";
import { sleep } from "./util";

const openAsync = promisify(fs.open);

export class ParityInstance {
    private process: ChildProcess;
    private dbDir: any;

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string
    ) {}

    public async start() {
        const bin = process.env.PARITY_BIN
            ? process.env.PARITY_BIN
            : this.projectRoot + "/blockchain_nodes/parity/parity";
        this.dbDir = tmp.dirSync();

        this.process = spawn(
            bin,
            [
                `--config=${this.projectRoot}/blockchain_nodes/parity/home/parity/.local/share/io.parity.ethereum/config.toml`,
                `--chain=${this.projectRoot}/blockchain_nodes/parity/home/parity/.local/share/io.parity.ethereum/chain.json`,
                `--base-path=${this.projectRoot}/blockchain_nodes/parity/home/parity/.local/share/io.parity.ethereum`,
                `--db-path=${this.dbDir.name}`,
                `--password=${this.projectRoot}/blockchain_nodes/parity/home/parity/authorities/authority.pwd`,
            ],

            {
                cwd: this.projectRoot,
                stdio: [
                    "ignore", // stdin
                    await openAsync(this.logDir + "/parity.log", "w"), // stdout
                    await openAsync(this.logDir + "/parity.log", "w"), // stderr
                ],
            }
        );

        this.process.on("exit", (code: number, signal: number) => {
            console.log(`parity exited with ${code || "signal " + signal}`);
        });
        await sleep(5000); // allow the nodes to start up
        return this;
    }

    public stop() {
        this.process.kill("SIGINT");
    }
}
