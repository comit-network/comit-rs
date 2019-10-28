import { Actor } from "./actor";

export class Actors {
    constructor(private readonly actors: Map<string, Actor>) {}

    get alice(): Actor {
        return this.getActorByName("alice");
    }

    get bob(): Actor {
        return this.getActorByName("bob");
    }

    private getActorByName(name: string): Actor {
        const maybeActor = this.actors.get(name);

        if (!maybeActor) {
            throw new Error(`Actor ${name} was not initialized`);
        }

        return maybeActor;
    }
}
