import { Actors } from "./actors";
import { Actor } from "./actors/actor";
import { HarnessGlobal } from "./utils";
import path from "path";

declare var global: HarnessGlobal;

export async function createActors(
    testName: string,
    actorNames: string[]
): Promise<Actors> {
    const actorsMap = new Map<string, Actor>();

    const listPromises: Promise<Actor>[] = [];
    for (const name of actorNames) {
        const cndLogFile = path.join(
            global.logRoot,
            "tests",
            testName,
            `cnd-${name}.log`
        );
        const actorLogger = global.log4js.getLogger(
            `tests/${testName}/${name}`
        );

        listPromises.push(
            Actor.newInstance(
                name,
                global.ledgerConfigs,
                global.projectRoot,
                cndLogFile,
                actorLogger
            )
        );
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
