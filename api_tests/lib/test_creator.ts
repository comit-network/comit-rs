import { expect, request } from "chai";
import { Action, EmbeddedRepresentationSubEntity, Entity } from "../gen/siren";
import { Actor } from "./actor";
import { ActionKind } from "./comit";
import "./setupChai";

export interface Test {
    /**
     * To be triggered once an action is executed
     */
    description: string;
    callback: (swapEntity: Entity) => Promise<void>;
}

export interface Step {
    /**
     * Triggers an action and do the callback
     *
     * @property actor: the actor for which/that triggers the action
     * @property action: the name of the action that will be extracted from the COMIT-rs HTTP API
     * @property waitUntil: a predicate passed on the test after the action is executed
     * @property test: a test to be executed after the action is executed, the body of a swap request is passed only if `state` property is set
     *
     */
    actor: Actor;
    action?: ActionKind;
    waitUntil?: (state: any) => boolean;
    test?: Test;
}

export function createTests(
    alice: Actor,
    bob: Actor,
    steps: Step[],
    initialUrl: string,
    listUrl: string,
    initialRequest: object
) {
    // This may need to become more generic at a later stage
    // However, it would be unnecessary pre-optimisation now.
    const swapLocations: { [key: string]: string } = {};

    it(
        "[alice] Should be able to make a request via HTTP api to " +
            initialUrl,
        async () => {
            const res: ChaiHttp.Response = await request(
                alice.comitNodeHttpApiUrl()
            )
                .post(initialUrl)
                .send(initialRequest);
            expect(res).to.have.status(201);
            const swapLocation: string = res.header.location;
            expect(swapLocation).to.not.be.empty;
            swapLocations.alice = swapLocation;
        }
    );

    it("[bob] Shows the Swap as IN_PROGRESS in " + listUrl, async () => {
        const swapEntity = await bob
            .pollComitNodeUntil(listUrl, body => body.entities.length > 0)
            .then(body => body.entities[0] as EmbeddedRepresentationSubEntity);

        expect(swapEntity.properties).to.have.property("protocol", "rfc003");
        expect(swapEntity.properties).to.have.property("status", "IN_PROGRESS");

        const selfLink = swapEntity.links.find(link =>
            link.rel.includes("self")
        );

        expect(selfLink).to.not.be.undefined;

        swapLocations.bob = selfLink.href;
    });

    while (steps.length !== 0) {
        const { action, actor, waitUntil, test } = steps.shift();

        let sirenAction: Action;

        if (action) {
            it(`[${actor.name}] has the ${action} action`, async function() {
                this.timeout(5000);

                sirenAction = await actor
                    .pollComitNodeUntil(
                        swapLocations[actor.name],
                        body =>
                            body.actions.findIndex(
                                candidate => candidate.name === action
                            ) !== -1
                    )
                    .then(body =>
                        body.actions.find(
                            candidate => candidate.name === action
                        )
                    );
            });

            it(`[${actor.name}] Can execute the ${action} action`, async function() {
                if (action === ActionKind.Refund) {
                    this.timeout(30000);
                } else {
                    this.timeout(5000);
                }

                await actor.doComitAction(sirenAction);
            });
        }

        if (waitUntil) {
            it(`[${actor.name}] transitions to correct state`, async function() {
                this.timeout(10000);
                await actor.pollComitNodeUntil(
                    swapLocations[actor.name],
                    body => waitUntil(body.properties.state)
                );
            });
        }

        if (test) {
            it(`[${actor.name}] ${test.description}`, async function() {
                this.timeout(10000);

                const body = await actor.pollComitNodeUntil(
                    swapLocations[actor.name],
                    () => true
                );

                return test.callback(body);
            });
        }
    }

    return swapLocations;
}
