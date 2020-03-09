import { Actors } from "./actors";
import { Actor } from "./actors/actor";
import { createActor } from "./create_actor";
import { HarnessGlobal, mkdirAsync, rimrafAsync } from "./utils";
import path from "path";

declare var global: HarnessGlobal;

export async function createActors(
    testName: string,
    actorNames: string[]
): Promise<Actors> {
    const actorsMap = new Map<string, Actor>();
    const testFolderName = path.join(global.logRoot, "tests", testName);

    await resetLogs(testFolderName);

    const listPromises: Promise<Actor>[] = [];
    for (const name of actorNames) {
        listPromises.push(createActor(testFolderName, name));
    }
    const createdActors = await Promise.all(listPromises);
    for (const actor of createdActors) {
        actorsMap.set(actor.getName(), actor);
    }

    const actors = new Actors(actorsMap);

    for (const name of actorNames) {
        actorsMap.get(name).actors = actors;
    }

    return actors;
}

async function resetLogs(logDir: string) {
    await rimrafAsync(logDir);
    await mkdirAsync(logDir, { recursive: true });
}
