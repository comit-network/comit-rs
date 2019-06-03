import { ActionKind } from "../lib/comit";
import { expect, request } from "chai";
import { Actor } from "../lib/actor";
import "chai/register-should";
import "../lib/setupChai";
import { Action, EmbeddedRepresentationSubEntity, Entity } from "../gen/siren";

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
    let swapLocations: { [key: string]: string } = {};

    it(
        "[alice] Should be able to make a request via HTTP api to " +
            initialUrl,
        async () => {
            let res: ChaiHttp.Response = await request(alice.comit_node_url())
                .post(initialUrl)
                .send(initialRequest);
            res.should.have.status(201);
            const swapLocation: string = res.header.location;
            swapLocation.should.not.be.empty;
            swapLocations["alice"] = swapLocation;
        }
    );

    it("[bob] Shows the Swap as IN_PROGRESS in " + listUrl, async () => {
        let swapEntity = await bob.pollComitNodeUntil(
            listUrl,
            body => body.entities.length > 0,
            body => body.entities[0] as EmbeddedRepresentationSubEntity
        );

        expect(swapEntity.properties).to.have.property("protocol", "rfc003");
        expect(swapEntity.properties).to.have.property("status", "IN_PROGRESS");

        let selfLink = swapEntity.links.find(link => link.rel.includes("self"));

        expect(selfLink).to.not.be.undefined;

        swapLocations["bob"] = selfLink.href;
    });

    while (steps.length !== 0) {
        let { action, actor, waitUntil, test } = steps.shift();

        let sirenAction: Action;

        if (action) {
            it(`[${actor.name}] has the ${action} action`, async function() {
                this.timeout(5000);

                sirenAction = await actor.pollComitNodeUntil(
                    swapLocations[actor.name],
                    body =>
                        body.actions.findIndex(
                            candidate => candidate.name === action
                        ) != -1,
                    body =>
                        body.actions.find(
                            candidate => candidate.name === action
                        )
                );
            });

            it(`[${
                actor.name
            }] Can execute the ${action} action`, async function() {
                if (action == ActionKind.Refund) {
                    this.timeout(30000);
                } else {
                    this.timeout(5000);
                }

                await actor.doComitAction(sirenAction);
            });
        }

        let body: Entity = null;
        if (waitUntil) {
            it(`[${
                actor.name
            }] transitions to correct state`, async function() {
                this.timeout(10000);
                body = await actor.pollComitNodeUntil(
                    swapLocations[actor.name],
                    body => waitUntil(body.properties.state)
                );
            });
        }

        if (test && body) {
            it(`[${actor.name}] ${test.description}`, async function() {
                this.timeout(10000);

                return test.callback(body);
            });
        }
    }
}
