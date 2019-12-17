import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { promisify } from "util";
import { sleep } from "./util";

const openAsync = promisify(fs.open);

export class ParityInstance {
    private process: ChildProcess;

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string
    ) {}

    public async start() {
        const bin = process.env.PARITY_BIN
            ? process.env.PARITY_BIN
            : this.projectRoot + "/blockchain_nodes/parity/parity";

        this.process = spawn(
            bin,
            [
                // ./parity
                // --config=./home/parity/.local/share/io.parity.ethereum/config.toml
                // --chain=./home/parity/.local/share/io.parity.ethereum/chain.json
                // --base-path=./home/parity/.local/share/io.parity.ethereum
                // --db-path=./home/parity/.local/share/io.parity.ethereum/chains
                // --password=./home/parity/authorities/authority.pwd
                `--config=${this.projectRoot}/blockchain_nodes/parity/home/parity/.local/share/io.parity.ethereum/config.toml`,
                `--chain=${this.projectRoot}/blockchain_nodes/parity/home/parity/.local/share/io.parity.ethereum/chain.json`,
                `--base-path=${this.projectRoot}/blockchain_nodes/parity/home/parity/.local/share/io.parity.ethereum`,
                `--db-path=${this.projectRoot}/blockchain_nodes/parity/home/parity/.local/share/io.parity.ethereum/chains`,
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
        console.log("Delete parity tmp dir...");
        // TODO remove dir
        // await sleep(1000);
        this.process.kill("SIGINT");
    }
}
