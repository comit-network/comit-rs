import { Actors } from "./actors";
import { createActors } from "./create_actors";
import { timeout } from "./utils";

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

        const actors = await createActors(`${name}.log`);

        try {
            await timeout(60000, testFn(actors));
        } finally {
            actors.alice.stop();
            actors.bob.stop();
        }
    });
}
