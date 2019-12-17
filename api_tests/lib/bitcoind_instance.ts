import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { promisify } from "util";
import { sleep } from "./util";

const openAsync = promisify(fs.open);

export class BitcoindInstance {
    private process: ChildProcess;

    constructor(
        private readonly projectRoot: string,
        private readonly logDir: string
    ) {}

    public async start() {
        // const bin = process.env.BITCOIND_BIN
        //     ? process.env.BITCOIND_BIN
        //     : this.projectRoot +
        //       "/blockchain_nodes/bitcoin-0.19.0.1/bin/bitcoind";

        const bin = "echo";
        this.process = spawn(
            bin,
            ["hallo"],
            // [
            //     `-datadir=${this.projectRoot}/blockchain_nodes/bitcoin-0.19.0.1/datadir`,
            //     "-regtest",
            //     "-server",
            //     "-printtoconsole",
            //     "-bind=0.0.0.0:18444",
            //     "-rpcbind=localhost:18443",
            //     "-rpcallowip=0.0.0.0/0",
            //     "-nodebug",
            //     "-acceptnonstdtxn=0",
            //     "-rest",
            // ],
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

        await sleep(10000); // allow the nodes to start up
        return this;
    }

    public stop() {
        console.log("Delete bitcoind tmp dir...");
        // TODO remove dir
        // await sleep(1000);
        this.process.kill("SIGINT");
    }
}
