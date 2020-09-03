import { ActorName, Actors } from "./actors";
import { Actor } from "./actors/actor";
import { HarnessGlobal } from "./utils";

declare var global: HarnessGlobal;

export async function createActors(
    testName: string,
    actorNames: ActorName[]
): Promise<Actors> {
    const actorsMap = new Map<string, Actor>();

    const listPromises: Promise<Actor>[] = [];
    for (const name of actorNames) {
        const cndLogFile = global.getLogFile([testName, `cnd-${name}.log`]);
        const actorLogger = global.getLogger([testName, name]);

        listPromises.push(
            Actor.newInstance(
                name,
                global.ledgerConfigs,
                global.cargoTargetDir,
                cndLogFile,
                actorLogger,
                global.cndConfigOverrides,
                global.gethLockDir,
                global.lndWallets
            )
        );
    }
    const createdActors = await Promise.all(listPromises);
    for (const actor of createdActors) {
        actorsMap.set(actor.name, actor);
    }

    const actors = new Actors(actorsMap);

    for (const name of actorNames) {
        actorsMap.get(name).actors = actors;
    }

    return actors;
}
