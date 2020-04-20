import { Actors } from "./actors";
import { createActors } from "./create_actors";
import pTimeout from "p-timeout";
import ProvidesCallback = jest.ProvidesCallback;

/*
 * This test function will take care of instantiating the actors and tearing them down again
 * after the test, regardless if the test succeeded or failed.
 */
function nActorTest(
    actorNames: ["alice", "bob", "charlie"] | ["alice", "bob"] | ["alice"],
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return async (done) => {
        // @ts-ignore
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

/*
 * Instantiates a new e2e test based on three actors
 *
 */
export function threeActorTest(
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return nActorTest(["alice", "bob", "charlie"], testFn);
}

/*
 * Instantiates a new e2e test based on two actors
 */
export function twoActorTest(
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return nActorTest(["alice", "bob"], testFn);
}

/*
 * Instantiates a new e2e test based on one actor
 */
export function oneActorTest(
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return nActorTest(["alice"], testFn);
}
