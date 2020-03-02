import { configure } from "log4js";
import { Actor } from "./actors/actor";
import { existsAsync, HarnessGlobal, mkdirAsync } from "./utils";
import path from "path";

declare var global: HarnessGlobal;

export async function createActor(
    logFilePath: string,
    name: string
): Promise<Actor> {
    const logRootForActor = path.join(global.logRoot, "tests", logFilePath);

    if (!(await existsAsync(logRootForActor))) {
        await mkdirAsync(logRootForActor, { recursive: true });
    }

    const loggerFactory = (whoAmI: string) =>
        configure({
            appenders: {
                file: {
                    type: "file",
                    filename: path.join(
                        "log",
                        "tests",
                        logFilePath,
                        "test.log"
                    ),
                },
            },
            categories: {
                default: { appenders: ["file"], level: "debug" },
            },
        }).getLogger(whoAmI);

    const actor = await Actor.newInstance(
        loggerFactory,
        name,
        global.ledgerConfigs,
        global.projectRoot,
        logRootForActor
    );

    return actor;
}
