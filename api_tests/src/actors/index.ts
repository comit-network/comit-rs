import { CndActor } from "./cnd_actor";

export type Role = "Alice" | "Bob";

export class Actors {
    constructor(private readonly actors: Map<Role, CndActor>) {}

    get alice(): CndActor {
        return this.getActorByRole("Alice");
    }

    get bob(): CndActor {
        return this.getActorByRole("Bob");
    }

    public getActorByRole(role: Role): CndActor {
        const maybeActor = this.actors.get(role);

        if (!maybeActor) {
            throw new Error(`Actor ${role} was not initialized`);
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
