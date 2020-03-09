import { configure } from "log4js";
import { Actor } from "./actors/actor";
import { HarnessGlobal } from "./utils";
import path from "path";

declare var global: HarnessGlobal;

export async function createActor(
    testFolderName: string,
    name: string
): Promise<Actor> {
    const loggerFactory = (whoAmI: string) =>
        configure({
            appenders: {
                file: {
                    type: "file",
                    filename: path.join(testFolderName, "test.log"),
                },
            },
            categories: {
                default: { appenders: ["file"], level: "debug" },
            },
        }).getLogger(whoAmI);

    return Actor.newInstance(
        loggerFactory,
        name,
        global.ledgerConfigs,
        global.projectRoot,
        testFolderName
    );
}
