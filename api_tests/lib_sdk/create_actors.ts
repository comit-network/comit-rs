import { Actors } from "./actors";
import { Actor } from "./actors/actor";
import { createActor } from "./create_actor";

export async function createActors(
    logFileName: string,
    actorNames: string[]
): Promise<Actors> {
    const actorsMap = new Map<string, Actor>();
    for (const name of actorNames) {
        actorsMap.set(name, await createActor(logFileName, name));
    }

    const actors = new Actors(actorsMap);

    for (const name of actorNames) {
        actorsMap.get(name).actors = actors;
    }

    return Promise.resolve(actors);
}
