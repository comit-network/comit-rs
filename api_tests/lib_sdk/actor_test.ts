import { Actors } from "./actors";
import { createActors } from "./create_actors";
import { timeout } from "./utils";
import { Actor } from "./actors/actor";
import { createActor } from "./create_actor";
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
    it(name, async function() {
        this.timeout(100_000); // absurd timeout. we have our own one further down
        const actors = await createActors(`${name}.log`, [
            "alice",
            "bob",
            "charlie",
        ]);

        try {
            await timeout(60000, testFn(actors));
        } catch (e) {
            await actors.alice.dumpState();
            await actors.bob.dumpState();
            await actors.charlie.dumpState();

            throw e;
        } finally {
            actors.alice.stop();
            actors.bob.stop();
            actors.charlie.stop();
        }
    });
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
        this.timeout(100_000); // absurd timeout. we have our own one further down

        const actors = await createActors(`${name}.log`, ["alice", "bob"]);

        try {
            await timeout(60000, testFn(actors));
        } catch (e) {
            await actors.alice.dumpState();
            await actors.bob.dumpState();

            throw e;
        } finally {
            actors.alice.stop();
            actors.bob.stop();
        }
    });
}

/*
 * Instantiates a new e2e test based on one actor
 *
 * This test function will take care of instantiating the actor and tearing it down again after the test, regardless if the test succeeded or failed.
 */
export function oneActorTest(
    name: string,
    testFn: (actor: Actor) => Promise<void>
) {
    it(name, async function() {
        this.timeout(100_000); // absurd timeout. we have our own one further down

        const alice = await createActor(`${name}.log`, "alice");
        try {
            await timeout(60000, testFn(alice));
        } catch (e) {
            await alice.dumpState();

            throw e;
        } finally {
            alice.stop();
        }
    });
}
