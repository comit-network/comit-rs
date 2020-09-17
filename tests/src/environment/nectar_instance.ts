import { JsonMap, stringify } from "@iarna/toml";
import { ChildProcess, spawn } from "child_process";
import tempWrite from "temp-write";
import waitForLogMessage from "./wait_for_log_message";
import { Logger } from "log4js";
import path from "path";
import { crashListener } from "./crash_listener";
import { openAsync, execAsync } from "./async_fs";
import { NectarConfig } from "./nectar_config";
import { sleep } from "../utils";
import { parseFixed } from "@ethersproject/bignumber";

export class NectarInstance {
    private process: ChildProcess;

    constructor(
        private readonly cargoTargetDirectory: string,
        private readonly logFile: string,
        private readonly logger: Logger,
        private readonly _config: NectarConfig
    ) {}

    public get config() {
        return this._config;
    }

    public async deposit(): Promise<DepositAddresses> {
        const bin = process.env.NECTAR_BIN
            ? process.env.NECTAR_BIN
            : path.join(this.cargoTargetDirectory, "debug", "nectar");

        this.logger.info("Using binary", bin);

        const configFile = await tempWrite(
            stringify((this._config as unknown) as JsonMap),
            "config.toml"
        );

        const { stdout } = await execAsync(
            `${bin} --config=${configFile} --network=dev deposit`,
            {
                encoding: "utf-8",
            }
        );

        return parseDepositOutput(stdout);
    }

    public async balance(): Promise<Balances> {
        const bin = process.env.NECTAR_BIN
            ? process.env.NECTAR_BIN
            : path.join(this.cargoTargetDirectory, "debug", "nectar");

        this.logger.info("Using binary", bin);

        const configFile = await tempWrite(
            stringify((this._config as unknown) as JsonMap),
            "config.toml"
        );

        const { stdout } = await execAsync(
            `${bin} --config=${configFile} --network=dev balance`,
            {
                encoding: "utf-8",
            }
        );

        return parseBalanceOutput(stdout);
    }

    /*
     * Executes nectar's `trade` command.
     *
     * Returns the PeerId under which this nectar instance participates in the COMIT network.
     */
    public async trade(): Promise<string> {
        const bin = process.env.NECTAR_BIN
            ? process.env.NECTAR_BIN
            : path.join(this.cargoTargetDirectory, "debug", "nectar");

        this.logger.info("Using binary", bin);

        const configFile = await tempWrite(
            stringify((this._config as unknown) as JsonMap),
            "config.toml"
        );

        const logFile = await openAsync(this.logFile, "w");
        this.process = spawn(
            bin,
            ["--config", configFile, "--network", "dev", "trade"],
            {
                cwd: this.cargoTargetDirectory,
                stdio: [
                    "ignore", // stdin
                    logFile, // stdout
                    logFile, // stderr
                ],
            }
        );

        this.process.on(
            "exit",
            crashListener(this.process.pid, "nectar", this.logFile)
        );

        const line = await waitForLogMessage(
            this.logFile,
            "Initialized swarm with identity"
        );

        // Nectar doesn't have an API like cnd does, so we extract the PeerId from the log!
        const match = line.match(/Initialized swarm with identity (.+)/);
        const peerId = match[1];

        // Give it some time to actually start
        await sleep(1000);

        this.logger.info("nectar started with PID", this.process.pid);

        return peerId;
    }

    public stop() {
        this.logger.info("Stopping nectar instance");

        this.process.kill("SIGINT");
        this.process = null;
    }

    public isRunning() {
        return this.process != null;
    }
}

interface DepositAddresses {
    bitcoin: string;
    ethereum: string;
}

export function parseDepositOutput(output: string): DepositAddresses {
    const match = output.match(
        /Bitcoin: (bcrt1[a-z0-9]+)\sDai\/Ether: (0x[a-z0-9]+)/
    );
    if (!match) {
        throw new Error("Failed to extract addresses from output");
    }

    return {
        bitcoin: match[1],
        ethereum: match[2],
    };
}

export interface Balances {
    bitcoin: bigint;
    ether: bigint;
    dai: bigint;
}

export function parseBalanceOutput(output: string): Balances {
    const match = output.match(
        /Bitcoin: ([0-9.]+) BTC\sDai: ([0-9.]+) DAI\sEther: ([0-9.]+) ETH/
    );
    if (!match) {
        throw new Error("Failed to extract addresses from output");
    }

    return {
        bitcoin: BigInt(parseFixed(match[1], 8).toString()),
        dai: BigInt(parseFixed(match[2], 18).toString()),
        ether: BigInt(parseFixed(match[3], 18)),
    };
}
