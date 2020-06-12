import { Actor } from "./actor";
import { Rfc003Actor } from "./rfc003_actor";

export class Actors {
    constructor(private readonly actors: Map<string, Actor>) {}

    get alice(): Actor {
        return this.getActorByName("alice");
    }

    get bob(): Actor {
        return this.getActorByName("bob");
    }

    get charlie(): Actor {
        return this.getActorByName("charlie");
    }

    public getActorByName(name: string): Actor {
        const maybeActor = this.actors.get(name);

        if (!maybeActor) {
            throw new Error(`Actor ${name} was not initialized`);
        }

        return maybeActor;
    }
}

export class Rfc003Actors {
    constructor(private readonly actors: Map<string, Rfc003Actor>) {}

    get alice(): Rfc003Actor {
        return this.getActorByName("alice");
    }

    get bob(): Rfc003Actor {
        return this.getActorByName("bob");
    }

    get charlie(): Rfc003Actor {
        return this.getActorByName("charlie");
    }

    public getActorByName(name: string): Rfc003Actor {
        const maybeActor = this.actors.get(name);

        if (!maybeActor) {
            throw new Error(`Actor ${name} was not initialized`);
        }

        return maybeActor;
    }
}
