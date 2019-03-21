import {
    AcceptRequestBody,
    ActionKind,
    ActionDirective,
    getMethod,
    HalResource,
    Method,
    SwapsResponse,
} from "../lib/comit";
import * as chai from "chai";
import { Actor } from "../lib/actor";
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
    action: ActionKind;
    requestBody?: AcceptRequestBody;
    uriQuery?: object;
    timeout?: number;
    state?: (state: any) => boolean;
    test?: Test;
}

async function getAction(
    actor: Actor,
    location: string,
    actionTrigger: ActionTrigger
): Promise<[string, ActionDirective]> {
    location.should.not.be.empty;

    const body = (await actor.pollComitNodeUntil(
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
        const res = await chai.request(actor.comit_node_url()).get(href);
        res.should.have.status(200);
        let payload = res.body;
        return [href, payload];
    }
    return [href, null];
}

async function executeAction(
    actor: Actor,
    actionTrigger: ActionTrigger,
    actionHref?: string,
    actionDirective?: ActionDirective
) {
    return (async function(method) {
        switch (method) {
            case Method.Get:
                return actor.do(actionDirective);
            case Method.Post:
                const res = await chai
                    .request(actor.comit_node_url())
                    .post(actionHref)
                    .send(actionTrigger.requestBody);
                res.should.have.status(200);
                return res;
            default:
                throw new Error("Unexpected error: unknown method");
        }
    })(getMethod(actionTrigger.action));
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
        let actionDirective: ActionDirective = null;
        let actionExecutionResult: any = null;

        it(
            "[" +
                action.actor.name +
                "] Can get the " +
                action.action +
                " action",
            async function() {
                this.timeout(action.timeout || 10000);
                [actionHref, actionDirective] = await getAction(
                    action.actor,
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
                actionExecutionResult = await executeAction(
                    action.actor,
                    action,
                    actionHref,
                    actionDirective
                );
            }
        );

        let body: any = null;
        if (action.state) {
            it(
                "[" + action.actor.name + "] state is as expected",
                async function() {
                    this.timeout(action.timeout || 10000);
                    body = (await action.actor.pollComitNodeUntil(
                        swapLocations[action.actor.name],
                        body => action.state(body.state)
                    )) as HalResource;
                }
            );
        }

        const test = action.test;
        if (test) {
            it(test.description, async function() {
                if (test.timeout) {
                    this.timeout(test.timeout);
                }

                return test.callback(body);
            });
        }
    }
}
