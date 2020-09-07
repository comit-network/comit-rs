import { Actor } from "./actor";

export type ActorName = "Alice" | "Bob";

export class Actors {
    constructor(private readonly actors: Map<string, Actor>) {}

    get alice(): Actor {
        return this.getActorByName("Alice");
    }

    get bob(): Actor {
        return this.getActorByName("Bob");
    }

    public getActorByName(name: ActorName): Actor {
        const maybeActor = this.actors.get(name);

        if (!maybeActor) {
            throw new Error(`Actor ${name} was not initialized`);
        }

        return maybeActor;
    }
}
