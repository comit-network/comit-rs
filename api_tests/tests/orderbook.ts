/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import OrderFactory from "../src/actors/order_factory";
import { twoActorTest } from "../src/actor_test";
import { Entity, Link } from "comit-sdk/dist/src/cnd/siren";
import { sleep } from "../src/utils";

it(
    "orderbook_bob_makes_order_alice_takes_order",
    twoActorTest(async ({ alice, bob }) => {
        // Get alice's listen address
        const aliceAddr = await alice.cnd.getPeerListenAddresses();

        // Get bobs's listen address
        const bobAddr = await bob.cnd.getPeerListenAddresses();

        // Bob dials alice
        // @ts-ignore
        await bob.cnd.client.post("dial", { addresses: aliceAddr });

        /// Wait for alice to accept an incoming connection from Bob
        await sleep(1000);

        const bobMakeOrderBody = OrderFactory.newHerc20HbitOrder(bobAddr[0]);
        // @ts-ignore
        // make response contain url in the header to the created order
        // poll this order to see when when it has been converted to a swap
        // "POST /orders"
        const bobMakeOrderResponse = await bob.cnd.client.post(
            "orders",
            bobMakeOrderBody
        );

        // Poll until Alice receives an order. The order must be the one that Bob created above.
        // @ts-ignore
        const aliceOrdersResponse = await alice.pollCndUntil<Entity>(
            "orders",
            (entity) => entity.entities.length > 0
        );
        const aliceOrderResponse: Entity = aliceOrdersResponse.entities[0];

        // Alice extracts the siren action to take the order
        const aliceOrderTakeAction = aliceOrderResponse.actions.find(
            (action: any) => action.name === "take"
        );
        // Alice executes the siren take action extracted in the previous line
        // The resolver function fills the refund and redeem address fields required
        // "POST /orders/63c0f8bd-beb2-4a9c-8591-a46f65913b0a/take"
        // Alice receives a url to the swap that was created as a result of taking the order
        // @ts-ignore
        const aliceTakeOrderResponse = await alice.cnd.executeSirenAction(
            aliceOrderTakeAction,
            async (field) => {
                //                const classes = field.class;

                if (field.name === "refund_identity") {
                    // @ts-ignore
                    return Promise.resolve(
                        "0x00a329c0648769a73afac7f9381e08fb43dbea72"
                    );
                }

                if (field.name === "redeem_identity") {
                    // @ts-ignore
                    return Promise.resolve(
                        "1F1tAaz5x1HUXrCNLbtMDqcw6o5GNn4xqX"
                    );
                }
            }
        );

        // Wait for bob to acknowledge that Alice has taken the order he created
        await sleep(1000);

        // @ts-ignore
        const aliceSwapResponse = await alice.cnd.client.get(
            aliceTakeOrderResponse.headers.location
        );
        expect(aliceSwapResponse.status).toEqual(200);

        // Since Alice has taken the swap, the order created by Bob should have an associated swap in the navigational link
        const bobGetOrderResponse = await bob.cnd.fetch<Entity>(
            bobMakeOrderResponse.headers.location
        );

        expect(bobGetOrderResponse.status).toEqual(200);
        const linkToBobSwap = bobGetOrderResponse.data.links.find(
            (link: Link) => link.rel.includes("swap")
        );
        expect(linkToBobSwap).toBeDefined();

        // The link the Bobs swap should return 200
        // "GET /swaps/934dd090-f8eb-4244-9aba-78e23d3f79eb HTTP/1.1"
        const bobSwapResponse = await bob.cnd.fetch<Entity>(linkToBobSwap.href);
        expect(bobSwapResponse.status).toEqual(200);

        // Bob and Alice both have a swap created from the order that Bob made and alice took.
    })
);
