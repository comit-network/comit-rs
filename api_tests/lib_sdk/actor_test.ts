import { Actors } from "./actors";
import { createActors } from "./create_actors";
import { timeout } from "./utils";

function nActorTest(
    name: string,
    actorNames: string[],
    testFn: (actors: Actors) => Promise<void>
) {
    it(name, async function() {
        this.timeout(100_000); // absurd timeout. we have our own one further down
        const actors = await createActors(`${name}.log`, actorNames);

        try {
            await timeout(60000, testFn(actors));
        } catch (e) {
            if (actors.alice) {
                await actors.alice.dumpState();
            }
            if (actors.bob) {
                await actors.bob.dumpState();
            }
            if (actors.charlie) {
                await actors.charlie.dumpState();
            }
            throw e;
        } finally {
            if (actors.alice) {
                await actors.alice.stop();
            }
            if (actors.bob) {
                await actors.bob.stop();
            }
            if (actors.charlie) {
                await actors.charlie.stop();
            }
        }
    });
}

/*
 * Instantiates a new e2e test based on three actors
 *
 * This test function will take care of instantiating the actors and tearing them down again
 * after the test, regardless if the test succeeded or failed.
 */
export function threeActorTest(
    name: string,
    testFn: (actors: Actors) => Promise<void>
) {
    nActorTest(name, ["alice", "bob", "charlie"], testFn);
}

/*
 * Instantiates a new e2e test based on two actors
 *
 * This test function will take care of instantiating the actors and tearing them down again
 * after the test, regardless if the test succeeded or failed.
 */
export function twoActorTest(
    name: string,
    testFn: (actors: Actors) => Promise<void>
) {
    it(name, async function() {
        nActorTest(name, ["alice", "bob"], testFn);
    });
}

/*
 * Instantiates a new e2e test based on one actor
 *
 * This test function will take care of instantiating the actor and tearing it down again after the test, regardless if the test succeeded or failed.
 */
export function oneActorTest(
    name: string,
    testFn: (actors: Actors) => Promise<void>
) {
    nActorTest(name, ["alice"], testFn);
}
