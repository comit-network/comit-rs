import { spawn } from "child_process";
import { Logger } from "log4js";
import { promises as asyncFs } from "fs";
import { existsAsync, openAsync } from "./async_fs";
import getPort from "get-port";
import { Startable } from "./index";

export default class FakeTreasuryServiceInstance implements Startable {
    constructor(
        private tsNode: string,
        private servicePath: string,
        private btcDaiPrice: number,
        private port: number,
        private pidFile: string,
        private logFile: string,
        private logger: Logger
    ) {}

    public static async new(
        tsNode: string,
        servicePath: string,
        btcDaiPrice: number,
        pidFile: string,
        logFile: string,
        logger: Logger
    ) {
        await existsAsync(tsNode);
        await existsAsync(servicePath);
        const port = await getPort();

        return new FakeTreasuryServiceInstance(
            tsNode,
            servicePath,
            btcDaiPrice,
            port,
            pidFile,
            logFile,
            logger
        );
    }

    public get host() {
        return `http://localhost:${this.port}`;
    }

    async start(): Promise<void> {
        const logFile = await openAsync(this.logFile, "w");
        const service = spawn(
            this.tsNode,
            [
                this.servicePath,
                this.btcDaiPrice.toString(),
                this.port.toString(),
            ],
            {
                stdio: ["ignore", logFile, logFile],
            }
        );

        await asyncFs.writeFile(this.pidFile, service.pid.toString(), {
            encoding: "utf-8",
        });

        service.unref();

        service.on("error", (error) => {
            this.logger.error("FakeTreasuryService throw an error", error);
        });

        service.on("exit", (code) => {
            this.logger.warn("FakeTreasuryService exited with", code);
        });
    }
}
