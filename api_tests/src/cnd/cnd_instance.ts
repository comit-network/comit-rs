import { JsonMap, stringify } from "@iarna/toml";
import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import tempWrite from "temp-write";
import { promisify } from "util";
import { CndConfigFile } from "../config";
import { sleep } from "../utils";
import waitForLogMessage from "../wait_for_log_message";
import { Logger } from "log4js";
import path from "path";

const openAsync = promisify(fs.open);

export class CndInstance {
    private process: ChildProcess;

    constructor(
        private readonly cargoTargetDirectory: string,
        private readonly logFile: string,
        private readonly logger: Logger,
        private readonly configFile: CndConfigFile
    ) {}

    public getConfigFile() {
        return this.configFile;
    }

    public async start() {
        const bin = process.env.CND_BIN
            ? process.env.CND_BIN
            : path.join(this.cargoTargetDirectory, "debug", "cnd");

        this.logger.info("Using binary", bin);

        const configFile = await tempWrite(
            stringify((this.configFile as unknown) as JsonMap),
            "config.toml"
        );

        this.process = spawn(bin, ["--config", configFile], {
            cwd: this.cargoTargetDirectory,
            stdio: [
                "ignore", // stdin
                await openAsync(this.logFile, "w"), // stdout
                await openAsync(this.logFile, "w"), // stderr
            ],
        });

        await waitForLogMessage(this.logFile, "Starting HTTP server on");

        // we emit the log _before_ we start the http server, let's make sure it actually starts up
        await sleep(1000);

        this.logger.info("cnd started with PID", this.process.pid);
    }

    public stop() {
        this.logger.info("Stopping cnd instance");

        this.process.kill("SIGINT");
        this.process = null;
    }

    public isRunning() {
        return this.process != null;
    }
}
