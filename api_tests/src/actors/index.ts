import { CndActor } from "./cnd_actor";

export type ActorName = "Alice" | "Bob";

export class Actors {
    constructor(private readonly actors: Map<string, CndActor>) {}

    get alice(): CndActor {
        return this.getActorByName("Alice");
    }

    get bob(): CndActor {
        return this.getActorByName("Bob");
    }

    public getActorByName(name: ActorName): CndActor {
        const maybeActor = this.actors.get(name);

        if (!maybeActor) {
            throw new Error(`Actor ${name} was not initialized`);
        }

        return maybeActor;
    }

    public async stop() {
        for (const actor of this.actors.values()) {
            await actor.stop();
        }
    }

    public async dumpState() {
        for (const actor of this.actors.values()) {
            await actor.dumpState();
        }
    }
}
