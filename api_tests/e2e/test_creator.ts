import {
    AcceptRequestBody,
    ActionKind,
    Action,
    getMethod,
    HalResource,
    Method,
    SwapsResponse,
} from "../lib/comit";
import * as chai from "chai";
import { Actor } from "../lib/actor";
import { sleep } from "../lib/util";
import * as URI from "urijs";

const should = chai.should();

interface Test {
    /**
     * To be triggered once an action is executed
     *
     * @property description: the description to use for the callback
     * @property callback: an (async) function take the body of a swap state response as parameter
     * @property timeout: if set, overrides the Mocha default timeout.
     */
    description: string;
    callback: any;
    timeout?: number;
}

interface ActionTrigger {
    /**
     * Triggers an action and do the callback
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
    requestBody?: AcceptRequestBody;
    uriQuery?: object;
    timeout?: number;
    state?: (state: any) => boolean;
    test?: Test;
}

async function getAction(
    location: string,
    actionTrigger: ActionTrigger
): Promise<[string, Action]> {
    location.should.not.be.empty;

    const body = (await actionTrigger.actor.pollComitNodeUntil(
        location,
        body => body._links[actionTrigger.action]
    )) as HalResource;

    let href: string = body._links[actionTrigger.action].href;
    href.should.not.be.empty;

    if (actionTrigger.uriQuery) {
        let hrefUri = new URI(href);
        hrefUri.addQuery(actionTrigger.uriQuery);
        href = hrefUri.toString();
    }

    if (getMethod(actionTrigger.action) === Method.Get) {
        const res = await chai
            .request(actionTrigger.actor.comit_node_url())
            .get(href);
        res.should.have.status(200);
        let payload = res.body;
        return [href, payload];
    }
    return [href, null];
}

function seconds_until(time: number): number {
    const diff = time - Math.floor(Date.now() / 1000);

    if (diff > 0) {
        return diff;
    } else {
        return 0;
    }
}

async function executeAction(
    actor: Actor,
    actionTrigger: ActionTrigger,
    actionHref?: string,
    actionDirective?: Action
) {
    const method = getMethod(actionTrigger.action);

    switch (method) {
        case Method.Get:
            await actor.do(actionDirective);
            break;
        case Method.Post:
            const res = await chai
                .request(actor.comit_node_url())
                .post(actionHref)
                .send(actionTrigger.requestBody);

            res.should.have.status(200);
            break;
        default:
            throw new Error(`unknown method: ${method}`);
    }
}

export async function createTests(
    alice: Actor,
    bob: Actor,
    actions: ActionTrigger[],
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
            let res: ChaiHttp.Response = await chai
                .request(alice.comit_node_url())
                .post(initialUrl)
                .send(initialRequest);
            res.should.have.status(201);
            const swapLocation: string = res.header.location;
            swapLocation.should.not.be.empty;
            swapLocations["alice"] = swapLocation;
        }
    );

    it("[bob] Shows the Swap as IN_PROGRESS in " + listUrl, async () => {
        let body = (await bob.pollComitNodeUntil(
            listUrl,
            body => body._embedded.swaps.length > 0
        )) as SwapsResponse;

        const swapEmbedded = body._embedded.swaps[0];
        swapEmbedded.protocol.should.equal("rfc003");
        swapEmbedded.status.should.equal("IN_PROGRESS");
        const swapLink = swapEmbedded._links;
        swapLink.should.be.a("object");
        const swapLocation: string = swapLink.self.href;
        swapLocation.should.not.be.empty;
        swapLocations["bob"] = swapLocation;
    });

    while (actions.length !== 0) {
        let action = actions.shift();
        let actionHref: string = null;
        let actionDirective: Action = null;
        const timeout = action.timeout || 10000;
        if (action.action) {
            it(
                "[" +
                    action.actor.name +
                    "] Can get the " +
                    action.action +
                    " action",
                async function() {
                    this.timeout(timeout);
                    [actionHref, actionDirective] = await getAction(
                        swapLocations[action.actor.name],
                        action
                    );
                }
            );

            it(
                "[" +
                    action.actor.name +
                    "] Can execute the " +
                    action.action +
                    " action",
                async function() {
                    if (actionDirective && actionDirective.invalid_until) {
                        const to_wait =
                            seconds_until(actionDirective.invalid_until) *
                                1000 +
                            1000; // Add an extra second for good measure
                        console.log(
                            `Waiting ${to_wait}ms for ${
                                action.actor.name
                            }'s  ‘${action.action}’ action to be ready`
                        );
                        this.timeout(to_wait + timeout);
                        await sleep(to_wait);
                    }

                    await executeAction(
                        action.actor,
                        action,
                        actionHref,
                        actionDirective
                    );
                }
            );
        }

        let body: any = null;
        if (action.state) {
            it(
                "[" + action.actor.name + "] transitions to correct state",
                async function() {
                    this.timeout(timeout);
                    body = (await action.actor.pollComitNodeUntil(
                        swapLocations[action.actor.name],
                        body => action.state(body.state)
                    )) as HalResource;
                }
            );
        }

        const test = action.test;
        if (test) {
            it(
                "[" + action.actor.name + "] " + test.description,
                async function() {
                    if (test.timeout) {
                        this.timeout(test.timeout);
                    }

                    return test.callback(body);
                }
            );
        }
    }
}
