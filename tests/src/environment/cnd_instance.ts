import { JsonMap, stringify } from "@iarna/toml";
import { ChildProcess, spawn } from "child_process";
import tempWrite from "temp-write";
import { CndConfigFile } from "./cnd_config_file";
import { sleep } from "../utils";
import waitForLogMessage from "./wait_for_log_message";
import { Logger } from "log4js";
import path from "path";
import { crashListener } from "./crash_listener";
import { openAsync } from "./async_fs";

export class CndInstance {
    private process: ChildProcess;

    constructor(
        private readonly cargoTargetDirectory: string,
        private readonly logFile: string,
        private readonly logger: Logger,
        private configFile: CndConfigFile
    ) {}

    public getConfigFile() {
        return this.configFile;
    }

    /**
     * Override the config of the node.
     */
    public setConfigFile(config: CndConfigFile) {
        this.configFile = config;
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

        const logFile = await openAsync(this.logFile, "w");
        this.process = spawn(
            bin,
            ["--config", configFile, "--network", "dev"],
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
            crashListener(this.process.pid, "cnd", this.logFile)
        );

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
