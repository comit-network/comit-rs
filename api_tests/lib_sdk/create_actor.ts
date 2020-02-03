import { configure } from "log4js";
import { HarnessGlobal } from "../lib/util";
import { Actor } from "./actors/actor";

declare var global: HarnessGlobal;

export async function createActor(
    logFileName: string,
    name: string = "alice"
): Promise<Actor> {
    const loggerFactory = (whoAmI: string) =>
        configure({
            appenders: {
                file: {
                    type: "file",
                    filename: "log/tests/" + logFileName,
                },
            },
            categories: {
                default: { appenders: ["file"], level: "debug" },
            },
        }).getLogger(whoAmI);

    const alice = await Actor.newInstance(
        loggerFactory,
        name,
        global.ledgerConfigs,
        global.projectRoot,
        global.logRoot
    );

    return Promise.resolve(alice);
}
