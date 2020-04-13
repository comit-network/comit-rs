import { spawn } from "child_process";
import { existsAsync, writeFileAsync } from "../utils";
import { Logger } from "log4js";

export default class BitcoinMinerInstance {
    public static async start(
        tsNode: string,
        minerPath: string,
        bitcoindConfigFile: string,
        pidFile: string,
        logger: Logger
    ) {
        if (!(await existsAsync(tsNode))) {
            throw new Error(`ts-node binary does not exist: ${tsNode}`);
        }

        if (!(await existsAsync(minerPath))) {
            throw new Error(`miner script does not exist: ${minerPath}`);
        }

        if (!(await existsAsync(bitcoindConfigFile))) {
            throw new Error(
                `bitcoind config file does not exist: ${bitcoindConfigFile}`
            );
        }

        const miner = spawn(tsNode, [minerPath, bitcoindConfigFile], {
            stdio: "ignore",
        });

        await writeFileAsync(pidFile, miner.pid, {
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
