import { expect, request } from "chai";
import { Response } from "superagent";
import { Action, EmbeddedRepresentationSubEntity, Entity } from "../gen/siren";
import { Actor } from "./actor";
import { ActionKind, LedgerAction } from "./comit";
import "./setup_chai";

export interface Test {
    description: string;
    callback: (swapEntity: Entity) => Promise<void>;
}

interface ArbitraryStepExecution {
    description: string;
    exec: (actor: Actor, swapHref: string) => Promise<void>;
}
interface StandardStepExecution {
    kind: ActionKind;
    test: (response: Response) => void;
}
type StepExecution =
    | ArbitraryStepExecution
    | StandardStepExecution
    | ActionKind;

function isStandardStepExecutionWithDefaultTest(
    action: StepExecution
): action is ActionKind {
    return typeof action === "string";
}

function isStandardStepExecutionWithCustomTest(
    action: StandardStepExecution | ArbitraryStepExecution
): action is StandardStepExecution {
    return "kind" in action && "test" in action;
}

function isArbitraryStepExecution(
    action: StandardStepExecution | ArbitraryStepExecution
): action is ArbitraryStepExecution {
    return "description" in action && "exec" in action;
}

export interface Step {
    actor: Actor;
    action?: StepExecution;
    waitUntil?: (state: any) => boolean;
    test?: Test;
}

interface SwapLocations {
    [key: string]: string;
}

export function createTests(
    alice: Actor,
    bob: Actor,
    steps: Step[],
    initialUrl: string,
    listUrl: string,
    initialRequest: object
) {
    const swapLocations: SwapLocations = {};

    it(
        "[alice] Should be able to make a request via HTTP api to " +
            initialUrl,
        async () => {
            const res: ChaiHttp.Response = await request(alice.cndHttpApiUrl())
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
            .pollCndUntil(listUrl, body => body.entities.length > 0)
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

        if (action) {
            if (isStandardStepExecutionWithDefaultTest(action)) {
                const defaultTest = (response: Response) =>
                    expect(response).to.have.status(200);

                standardActionSteps(actor, swapLocations, action, defaultTest);
            } else if (isStandardStepExecutionWithCustomTest(action)) {
                standardActionSteps(
                    actor,
                    swapLocations,
                    action.kind,
                    action.test
                );
            } else if (isArbitraryStepExecution(action)) {
                it(`[${actor.name}] ${action.description}`, async function() {
                    this.timeout(10000);
                    await action.exec(actor, swapLocations[actor.name]);
                });
            }
        }

        if (waitUntil) {
            it(`[${actor.name}] transitions to correct state`, async function() {
                this.timeout(10000);
                await actor.pollCndUntil(
                    swapLocations[actor.name],
                    body => waitUntil(body.properties.state)
                );
            });
        }

        if (test) {
            it(`[${actor.name}] ${test.description}`, async function() {
                this.timeout(10000);

                const body = await actor.pollCndUntil(
                    swapLocations[actor.name],
                    () => true
                );

                return test.callback(body);
            });
        }
    }

    return swapLocations;
}

export function hasAction(actionKind: ActionKind) {
    return (body: Entity) =>
        body.actions.findIndex(candidate => candidate.name === actionKind) !==
        -1;
}

export function mapToAction(actionKind: ActionKind): (body: Entity) => Action {
    return (body: Entity) =>
        body.actions.find(candidate => candidate.name === actionKind);
}

function standardActionSteps(
    actor: Actor,
    swapLocations: SwapLocations,
    actionKind: ActionKind,
    actionTest: (response: Response) => void
) {
    let sirenAction: Action;

    it(`[${actor.name}] has the ${actionKind} action`, async function() {
        this.timeout(5000);

        sirenAction = await actor
            .pollCndUntil(
                swapLocations[actor.name],
                hasAction(actionKind)
            )
            .then(mapToAction(actionKind));
    });

    it(`[${actor.name}] Can execute the ${actionKind} action`, async function() {
        if (actionKind === ActionKind.Refund) {
            this.timeout(30000);
        } else {
            this.timeout(5000);
        }

        const response = await actor.doComitAction(sirenAction);

        actionTest(response);

        // We should check against our own content type here to describe "LedgerActions"
        // Don't take it literally but something like `application/vnd.comit-ledger-action+json`
        // For now, checking for `application/json` + the fields should do the job as well because accept & decline don't return a body
        if (
            response.type === "application/json" &&
            response.body &&
            response.body.type &&
            response.body.payload
        ) {
            const body = response.body as LedgerAction;

            await actor.doLedgerAction(body);
        }
    });
}
