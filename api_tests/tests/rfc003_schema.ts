import { Actor } from "../src/actors/actor";
import * as sirenJsonSchema from "../siren.schema.json";
import * as swapPropertiesJsonSchema from "../swap.schema.json";
import { twoActorTest } from "../src/actor_test";
import { createDefaultSwapRequest, DEFAULT_ALPHA } from "../src/utils";
import { Action, EmbeddedRepresentationSubEntity, Link } from "comit-sdk";
import "../src/schema_matcher";
import { Rfc003Actor } from "../src/actors/rfc003_actor";

// ******************************************** //
// RFC003 schema tests                          //
// ******************************************** //

describe("Rfc003 schema tests", () => {
    it(
        "get-all-swaps-is-valid-siren",
        twoActorTest(async ({ alice }) => {
            const res = await alice.cnd.fetch("/swaps");

            expect(res.data).toMatchSchema(sirenJsonSchema);
        })
    );

    it(
        "get-single-swap-contains-link-to-rfc",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
            // Alice send swap request to Bob
            await alice.actor.cnd.postSwap(await createDefaultSwapRequest(bob));

            const aliceSwapEntity = await alice
                .pollCndUntil("/swaps", (body) => body.entities.length > 0)
                .then(
                    (body) =>
                        body.entities[0] as EmbeddedRepresentationSubEntity
                );

            const protocolLink = aliceSwapEntity.links.find((link: Link) =>
                link.rel.includes("describedby")
            );

            expect(protocolLink).toStrictEqual({
                rel: ["describedby"],
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

    expect(body.entities.length).toBeGreaterThan(0);

    body.entities.map((entity) => {
        expect(entity.properties).toHaveProperty("status", "IN_PROGRESS");
    });
}

describe("Rfc003 schema swap reject tests", () => {
    it(
        "alice-can-make-default-swap-request",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
            // Alice should be able to send two swap requests to Bob
            const url1 = await alice.actor.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.reasonable,
                },
            });

            const url2 = await alice.actor.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.stingy,
                },
            });

            await assertSwapsInProgress(alice.actor);

            // make sure bob processed the swaps fully
            await bob.pollSwapDetails(url1);
            await bob.pollSwapDetails(url2);

            await assertSwapsInProgress(bob.actor);
        })
    );

    it(
        "bob-can-decline-swap",
        twoActorTest(async (actors) => {
            const [alice, bob] = Rfc003Actor.convert([
                actors.alice,
                actors.bob,
            ]);
            // Alice should be able to send two swap requests to Bob
            const aliceReasonableSwap = await alice.actor.cnd.postSwap({
                ...(await createDefaultSwapRequest(bob)),
                alpha_asset: {
                    name: DEFAULT_ALPHA.asset.name,
                    quantity: DEFAULT_ALPHA.asset.quantity.reasonable,
                },
            });

            const aliceStingySwap = await alice.actor.cnd.postSwap({
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
            expect(bobSwapDetails.actions.map((action) => action.name)).toEqual(
                expect.arrayContaining(["accept", "decline"])
            );

            /// Decline the swap
            const decline = bobSwapDetails.actions.find(
                (action: Action) => action.name === "decline"
            );
            const declineRes = await bob.actor.cnd.executeSirenAction(decline);

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
