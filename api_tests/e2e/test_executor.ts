import { ActionDirective, HalResource, SwapsResponse } from "../lib/comit";
import * as chai from "chai";
import { Actor } from "../lib/actor";

const should = chai.should();

export enum Method {
    Get,
    Post,
}

export class AfterTest {
    /**
     * To be triggered once an action is executed
     *
     * @param description: the description to use for the test
     * @param callback: an (async) function takes 3 parameters:
     *                  - the array of swapLocations
     *                  - the value returned by `getAction`
     *                  - the value returned by `executeAction`
     * @param timeout: if set, overrides the Mocha default timeout.
     */
    description: string;
    callback: any;
    timeout?: number;

    constructor(description: string, callback: any, timeout?: number) {
        this.description = description;
        this.callback = callback;
        if (timeout) {
            this.timeout = timeout;
        }
    }
}

export class ActionTrigger {
    /**
     * Triggers an action and do the afterTest
     *
     * @param actor: the actor for which/that triggers the action
     * @param name: the name of the action that will be extracted from the COMIT-rs HTTP API
     * @param payload: the payload to pass if the action requires a POST call on the COMIT-rs HTTP API
     * @param parameters: the GET parameters to pass if the action requires a GET call on the COMIT-rs HTTP API
     * @param method: the HTTP Method to use on the action.
     * @param timeout: the time to allow the action to be executed
     * @param afterTest: a afterTest to be executed after the action is executed.
     *
     */
    actor: Actor;
    name: string;
    method: Method;
    payload?: object;
    parameters?: string;
    timeout?: number;
    afterTest?: AfterTest;

    constructor({
        actor,
        name,
        method,
        timeout,
        payload,
        parameters,
        afterTest,
    }: ActionTrigger) {
        this.actor = actor;
        this.name = name;
        this.method = method;
        this.timeout = timeout;
        switch (method) {
            case Method.Post: {
                if (payload) {
                    this.payload = payload;
                }
                break;
            }
            case Method.Get:
                {
                    if (parameters) {
                        this.parameters = parameters;
                    }
                }
                break;
            default:
                break;
        }
        if (afterTest) {
            this.afterTest = afterTest;
        }
    }
}

async function getAction(
    actor: Actor,
    location: string,
    actionTrigger: ActionTrigger
): Promise<[string, ActionDirective]> {
    location.should.not.be.empty;

    const body = (await actor.pollComitNodeUntil(
        location,
        body => body._links[actionTrigger.name]
    )) as HalResource;

    let href: string = body._links[actionTrigger.name].href;
    href.should.not.be.empty;

    if (actionTrigger.parameters) {
        href = href + "?" + actionTrigger.parameters;
    }

    if (actionTrigger.method === Method.Get) {
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
    actionPayload?: ActionDirective
) {
    return (async function(method) {
        switch (method) {
            case Method.Get:
                return actor.do(actionPayload);
            case Method.Post:
                const res = await chai
                    .request(actor.comit_node_url())
                    .post(actionHref)
                    .send(actionTrigger.payload);
                res.should.have.status(200);
                return res;
            default:
                throw new Error("Unexpected error: unknown method");
        }
    })(actionTrigger.method);
}

export async function execute(
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

    it("[alice] Should be able to make a request via HTTP api", async () => {
        let res: ChaiHttp.Response = await chai
            .request(alice.comit_node_url())
            .post(initialUrl)
            .send(initialRequest);

        res.should.have.status(201);
        const swapLocation: string = res.header.location;
        swapLocation.should.not.be.empty;
        swapLocations["alice"] = swapLocation;
    });

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
        swapLocations["bob"] = swapLink.self.href;
        swapLocations["bob"].should.be.a("string");
    });

    while (actions.length !== 0) {
        let action = actions.shift();
        let actionHref: string = null;
        let actionPayload: ActionDirective = null;
        let actionExecutionResult: any = null;

        it(
            "[" +
                action.actor.name +
                "] Can get the " +
                action.name +
                " action",
            async function() {
                this.timeout(action.timeout);
                [actionHref, actionPayload] = await getAction(
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
                action.name +
                " action",
            async function() {
                actionExecutionResult = await executeAction(
                    action.actor,
                    action,
                    actionHref,
                    actionPayload
                );
            }
        );

        const afterTest = action.afterTest;
        if (afterTest) {
            it(afterTest.description, async function() {
                if (afterTest.timeout) {
                    this.timeout(afterTest.timeout);
                }
                return afterTest.callback(
                    swapLocations,
                    [actionHref, actionPayload],
                    actionExecutionResult
                );
            });
        }
    }
}
