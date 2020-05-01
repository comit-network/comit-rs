import { Actor } from "../src/actors/actor";
import * as sirenJsonSchema from "../siren.schema.json";
import * as swapPropertiesJsonSchema from "../swap.schema.json";
import { twoActorTest } from "../src/actor_test";
import { createDefaultSwapRequest, DEFAULT_ALPHA } from "../src/utils";
import {
    Action,
    EmbeddedRepresentationSubEntity,
    Entity,
    Link,
} from "comit-sdk";
import { extendSchemaMatcher } from "../src/schema_matcher";

extendSchemaMatcher();

// ******************************************** //
// RFC003 schema tests                          //
// ******************************************** //

async function assertValidSirenDocument(swapsEntity: Entity, alice: Actor) {
    const selfLink = swapsEntity.links.find((link: Link) =>
        link.rel.includes("self")
    ).href;

    const swapResponse = await alice.cnd.fetch(selfLink);
    const swapEntity = swapResponse.data as Entity;

    expect(swapEntity).toMatchSchema(sirenJsonSchema);
    expect(swapEntity.properties).toMatchSchema(swapPropertiesJsonSchema);
}

describe("Rfc003 schema tests", () => {
    it(
        "get-all-swaps-is-valid-siren",
        twoActorTest(async ({ alice }) => {
            const res = await alice.cnd.fetch("/swaps");

            expect(res.data).toMatchSchema(sirenJsonSchema);
        })
    );

    it(
        "get-single-swap-is-valid-siren",
        twoActorTest(async ({ alice, bob }) => {
            // Alice send swap request to Bob
            await alice.cnd.postSwap(await createDefaultSwapRequest(bob));

            const aliceSwapEntity = await alice
                .pollCndUntil("/swaps", (body) => body.entities.length > 0)
                .then(
                    (body) =>
                        body.entities[0] as EmbeddedRepresentationSubEntity
                );

            await assertValidSirenDocument(aliceSwapEntity, alice);

            const bobsSwapEntity = await bob
                .pollCndUntil("/swaps", (body) => body.entities.length > 0)
                .then(
                    (body) =>
                        body.entities[0] as EmbeddedRepresentationSubEntity
                );
            await assertValidSirenDocument(bobsSwapEntity, bob);
        })
    );

    it(
        "get-single-swap-contains-link-to-rfc",
        twoActorTest(async ({ alice, bob }) => {
            // Alice send swap request to Bob
            await alice.cnd.postSwap(await createDefaultSwapRequest(bob));

            const aliceSwapEntity = await alice
                .pollCndUntil("/swaps", (body) => body.entities.length > 0)
                .then(
                    (body) =>
                        body.entities[0] as EmbeddedRepresentationSubEntity
                );

            const protocolLink = aliceSwapEntity.links.find((link: Link) =>
                link.rel.includes("describedBy")
            );

            expect(protocolLink).toStrictEqual({
                rel: ["describedBy"],
                class: ["protocol-spec"],
                type: "text/html",
                href:
                    "https://github.com/comit-network/RFCs/blob/master/RFC-003-SWAP-Basic.adoc",
            });
        })
    );
});

// ******************************************** //
// RFC003 Swap Reject                           //
// ******************************************** //

async function assertSwapsInProgress(actor: Actor) {
    const res = await actor.cnd.fetch("/swaps");
    const body = res.data as { entities: EmbeddedRepresentationSubEntity[] };
    expect.assertions(body.entities.length);

    body.entities.map((entity) => {
        expect(entity.properties).toHaveProperty("status", "IN_PROGRESS");
    });
}

describe("Rfc003 schema swap reject tests", () => {
    it(
        "alice-can-make-default-swap-request",
        twoActorTest(async ({ alice, bob }) => {
            // Alice should be able to send two swap requests to Bob
            const url1 = await alice.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.reasonable,
                },
            });

            const url2 = await alice.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.stingy,
                },
            });

            await assertSwapsInProgress(alice);

            // make sure bob processed the swaps fully
            await bob.pollSwapDetails(url1);
            await bob.pollSwapDetails(url2);

            await assertSwapsInProgress(bob);
        })
    );

    it(
        "bob-can-decline-swap",
        twoActorTest(async ({ alice, bob }) => {
            // Alice should be able to send two swap requests to Bob
            const aliceReasonableSwap = await alice.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.reasonable,
                },
            });

            const aliceStingySwap = await alice.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.stingy,
                },
            });

            const bobSwapDetails = await bob.pollSwapDetails(aliceStingySwap);

            expect(bobSwapDetails.properties).toMatchSchema(
                swapPropertiesJsonSchema
            );
            expect(bobSwapDetails.actions).toEqual(
                expect.arrayContaining([
                    {
                        name: "accept",
                    },
                    {
                        name: "decline",
                    },
                ])
            );

            /// Decline the swap
            const decline = bobSwapDetails.actions.find(
                (action: Action) => action.name === "decline"
            );
            const declineRes = await bob.cnd.executeSirenAction(decline);

            expect(declineRes.status).toBe(200);

            const bobPollPromise = bob.pollCndUntil(
                aliceStingySwap,
                (entity) =>
                    entity.properties.state.communication.status === "DECLINED"
            );
            await expect(bobPollPromise).resolves.toBeDefined();

            const aliceReasonableSwapDetails = await alice.pollSwapDetails(
                aliceReasonableSwap
            );

            const alicePollPromise = alice.pollCndUntil(
                aliceStingySwap,
                (entity) =>
                    entity.properties.state.communication.status === "DECLINED"
            );

            await expect(alicePollPromise).resolves.toBeDefined();

            expect(
                aliceReasonableSwapDetails.properties.state.communication.status
            ).toBe("SENT");
        })
    );
});
