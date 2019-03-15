import { ActionDirective, SwapResponse, SwapsResponse } from "../lib/comit";
import * as chai from "chai";
import { Actor } from "../lib/actor";
import AsyncFunc = Mocha.AsyncFunc;

const should = chai.should();

export enum Method {
    Get,
    Post,
}

export class Callback {
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
     * Triggers an action and do the callback
     *
     * @param actor: the actor for which/that triggers the action
     * @param name: the name of the action that will be extracted from the COMIT-rs HTTP API
     * @param payload: the payload to pass if the action requires a POST call on the COMIT-rs HTTP API
     * @param parameters: the GET parameters to pass if the action requires a GET call on the COMIT-rs HTTP API
     * @param method: the HTTP Method to use on the action.
     * @param timeout: the time to allow the action to be executed
     * @param callback: a callback to be executed after the action is executed.
     *
     */
    actor: Actor;
    name: string;
    method: Method;
    payload?: object;
    parameters: string;
    timeout: number;
    callback: Callback;

    constructor(
        actor: Actor,
        name: string,
        method: Method,
        timeout: number = 10000,
        payload?: object,
        parameters?: string,
        callback?: Callback
    ) {
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
        if (callback) {
            this.callback = callback;
        }
    }
}

async function getAction(
    actor: Actor,
    location: string,
    actionTrigger: ActionTrigger
): Promise<[string, ActionDirective]> {
    location.should.not.be.empty;

    let [href, action] = await actor
        .pollComitNodeUntil(location, body => body._links[actionTrigger.name])
        .then(async function(body: SwapResponse) {
            let href: string = body._links[actionTrigger.name].href;
            href.should.not.be.empty;

            if (actionTrigger.parameters) {
                href = href + "?" + actionTrigger.parameters;
            }

            if (actionTrigger.method === Method.Get) {
                return chai
                    .request(actor.comit_node_url())
                    .get(href)
                    .then(res => {
                        res.should.have.status(200);
                        let payload = res.body;
                        return [href, payload];
                    });
            }
            return [href, null];
        });

    return [href, action];
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
                return chai
                    .request(actor.comit_node_url())
                    .post(actionHref)
                    .send(actionTrigger.payload)
                    .then(res => {
                        res.should.have.status(200);
                    });
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
    initialRequest: object
) {
    // TODO: Make this section more generic
    let swapLocations: { [key: string]: string } = {};

    it("[Alice] Should be able to make a request via HTTP api", async () => {
        let res: ChaiHttp.Response = await chai
            .request(alice.comit_node_url())
            .post(initialUrl)
            .send(initialRequest);

        res.should.have.status(201);
        const swapLocation: string = res.header.location;
        swapLocation.should.not.be.empty;
        swapLocations["alice"] = swapLocation;
    });

    it("[Alice] Should be in IN_PROGRESS and SENT after sending the request to Bob", async function() {
        this.timeout(10000);
        await alice.pollComitNodeUntil(
            swapLocations["alice"],
            body =>
                body.status === "IN_PROGRESS" &&
                body.state.communication.status === "SENT"
        );
    });

    it("[Bob] Shows the Swap as IN_PROGRESS in /swaps", async () => {
        let body = (await bob.pollComitNodeUntil(
            "/swaps",
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

        const callback = action.callback;
        if (callback) {
            it(callback.description, async function() {
                if (callback.timeout) {
                    this.timeout(callback.timeout);
                }
                return callback.callback(
                    swapLocations,
                    [actionHref, actionPayload],
                    actionExecutionResult
                );
            });
        }
    }
}
