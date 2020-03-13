import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tmp from "tmp";
import { LogReader } from "./log_reader";
import { promisify } from "util";
import { sleep } from "../utils";
import getPort from "get-port";
import { EthereumInstance } from "./ethereum";

const openAsync = promisify(fs.open);

export class ParityInstance implements EthereumInstance {
    private process: ChildProcess;
    private dbDir: any;

    public static async new(projectRoot: string, logDir: string) {
        return new ParityInstance(
            projectRoot,
            logDir,
            await getPort({ port: 8545 }),
            await getPort()
        );
    }

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string,
        public readonly rpcPort: number,
        public readonly p2pPort: number
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
                `--jsonrpc-port=${this.rpcPort}`,
                `--port=${this.p2pPort}`,
                `--no-ws`,
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
            console.log(`parity exited with ${code || `signal ${signal}`}`);
        });
        const logReader = new LogReader(this.logDir + "/parity.log");
        await logReader.waitForLogMessage("Public node URL:");

        console.log(`parity started with PID ${this.process.pid}`);
    }

    public get rpcUrl() {
        return `http://localhost:${this.rpcPort}`;
    }

    public async stop() {
        this.process.kill("SIGTERM");
        await sleep(3000);
        this.process.kill("SIGKILL");
    }
}
