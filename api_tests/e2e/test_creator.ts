import { ActionKind } from "../lib/comit";
import { expect, request } from "chai";
import { Actor } from "../lib/actor";
import "chai/register-should";
import "../lib/setupChai";
import { Action, EmbeddedRepresentationSubEntity } from "../gen/siren";

export interface Test {
    /**
     * To be triggered once an action is executed
     *
     * @property description: the description to use for the callback
     * @property callback: an (async) function take the body of a swap state response as parameter
     * @property timeoutOverride: if set, overrides the Mocha default timeout.
     */
    description: string;
    callback: (body: any) => Promise<void>;
    timeoutOverride?: number;
}

export interface ActionTrigger {
    /**
     * Triggers an action and doLedgerAction the callback
     *
     * @property actor: the actor for which/that triggers the action
     * @property action: the name of the action that will be extracted from the COMIT-rs HTTP API
     * @property requestBody: the requestBody to pass if the action requires a POST call on the COMIT-rs HTTP API
     * @property uriQuery: the GET parameters to pass if the action requires a GET call on the COMIT-rs HTTP API
     * @property timeout: the time to allow the action to be executed
     * @property state: a predicate passed on the test after the action is executed
     * @property test: a test to be executed after the action is executed, the body of a swap request is passed only if `state` property is set
     *
     */
    actor: Actor;
    action?: ActionKind;
    state?: (state: any) => boolean;
    test?: Test;
}

export function createTests(
    alice: Actor,
    bob: Actor,
    actionTriggers: ActionTrigger[],
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
        let swapsEntity = await bob.pollComitNodeUntil(
            listUrl,
            body => body.entities.length > 0
        );

        let swapEntity = swapsEntity
            .entities[0] as EmbeddedRepresentationSubEntity;

        expect(swapEntity.properties).to.have.property("protocol", "rfc003");
        expect(swapEntity.properties).to.have.property("status", "IN_PROGRESS");

        let selfLink = swapEntity.links.find(link => link.rel.includes("self"));

        expect(selfLink).to.not.be.undefined;

        swapLocations["bob"] = selfLink.href;
    });

    while (actionTriggers.length !== 0) {
        let { action, actor, state, test } = actionTriggers.shift();

        let sirenAction: Action;

        if (action) {
            it(`[${actor.name}] has the ${action} action`, async function() {
                this.timeout(5000);

                let body = await actor.pollComitNodeUntil(
                    swapLocations[actor.name],
                    body =>
                        body.actions.findIndex(
                            candidate => candidate.name === action
                        ) != -1
                );

                sirenAction = body.actions.find(
                    candidate => candidate.name === action
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

        let body: any = null;
        if (state) {
            it(`[${
                actor.name
            }] transitions to correct state`, async function() {
                this.timeout(10000);
                body = await actor.pollComitNodeUntil(
                    swapLocations[actor.name],
                    body => state(body.properties.state)
                );
            });
        }

        if (test) {
            it(`[${actor.name}] ${test.description}`, async function() {
                let timeoutOverride = test.timeoutOverride;
                this.timeout(timeoutOverride ? timeoutOverride : 10000);

                return test.callback(body);
            });
        }
    }
}
