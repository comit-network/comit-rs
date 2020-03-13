import { Actor } from "./actors/actor";
import { HarnessGlobal } from "./utils";

declare var global: HarnessGlobal;

export async function createActor(
    testFolderName: string,
    name: string
): Promise<Actor> {
    return Actor.newInstance(
        name,
        global.ledgerConfigs,
        global.projectRoot,
        testFolderName
    );
}
