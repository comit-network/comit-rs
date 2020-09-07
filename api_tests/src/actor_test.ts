import { ActorName, Actors } from "./actors";
import pTimeout from "p-timeout";
import { HarnessGlobal } from "./environment";
import { Actor } from "./actors/actor";
import ProvidesCallback = jest.ProvidesCallback;

declare var global: HarnessGlobal;

/*
 * Instantiates a new e2e test based on one actor, Alice.
 */
export function oneActorTest(
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return nActorTest(["Alice"], testFn);
}

/*
 * Instantiates a new e2e test based on two actors, Alice and Bob.
 */
export function twoActorTest(
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return nActorTest(["Alice", "Bob"], testFn);
}

async function createActors(
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

/*
 * This test function will take care of instantiating the actors and tearing them down again
 * after the test, regardless if the test succeeded or failed.
 */
function nActorTest(
    actorNames: ["Alice"] | ["Alice", "Bob"],
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return async (done) => {
        const name = jasmine.currentTestName;
        if (!name.match(/[A-z0-9\-]+/)) {
            // We use the test name as a file name for the log and hence need to restrict it.
            throw new Error(
                `Testname '${name}' is invalid. Only A-z, 0-9 and dashes are allowed.`
            );
        }

        const actors = await createActors(name, actorNames);

        try {
            await pTimeout(testFn(actors), 120_000);
        } catch (e) {
            for (const actorName of actorNames) {
                await actors.getActorByName(actorName).dumpState();
            }
            throw e;
        } finally {
            for (const actorName of actorNames) {
                const actor = actors.getActorByName(actorName);
                await actor.stop();
            }
        }
        done();
    };
}
