import { spawn } from "child_process";
import { Logger } from "log4js";
import { promises as asyncFs } from "fs";
import { existsAsync } from "./async_fs";

export default class BitcoinMinerInstance {
    public static async start(
        tsNode: string,
        minerPath: string,
        bitcoindConfigFile: string,
        pidFile: string,
        logger: Logger
    ) {
        await existsAsync(tsNode);
        await existsAsync(minerPath);
        await existsAsync(bitcoindConfigFile);

        const miner = spawn(tsNode, [minerPath, bitcoindConfigFile], {
            stdio: "ignore",
        });

        await asyncFs.writeFile(pidFile, miner.pid.toString(), {
            encoding: "utf-8",
        });

        miner.unref();

        miner.on("error", (error) => {
            logger.error("bitcoin miner threw an error ", error);
        });

        miner.on("exit", (code) => {
            logger.warn("bitcoin miner exited with code ", code);
        });
    }
}
