import { configure } from "log4js";
import { Actor } from "./actors/actor";
import { HarnessGlobal } from "./utils";

declare var global: HarnessGlobal;

export async function createActor(
    logFileName: string,
    name: string
): Promise<Actor> {
    const loggerFactory = (whoAmI: string) =>
        configure({
            appenders: {
                file: {
                    type: "file",
                    filename: "log/tests/" + logFileName.replace(/\//g, "_"),
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
        global.logRoot
    );

    return actor;
}
