import { oneActorTest, twoActorTest } from "../src/actor_test";
import SwapFactory from "../src/actors/swap_factory";
import {
    HalightLightningBitcoinHerc20EthereumErc20RequestBody,
    Herc20EthereumErc20HalightLightningBitcoinRequestBody,
} from "comit-sdk";

// ******************************************** //
// Lightning routes                               //
// ******************************************** //

describe("Lightning routes tests", () => {
    it(
        "create-herc20-ethereum-erc20-halight-lightning-bitcoin-returns-bad-request",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob, true))
                .herc20EthereumErc20HalightLightningBitcoin;

            const expectedProblem = {
                status: 400,
                title: "lightning is not configured.",
                detail:
                    "lightning ledger is not properly configured, swap involving this ledger are not available.",
            };

            await expect(
                alice.cnd.createHerc20EthereumErc20HalightLightningBitcoin(
                    bodies.alice
                )
            ).rejects.toMatchObject(expectedProblem);
            await expect(
                bob.cnd.createHerc20EthereumErc20HalightLightningBitcoin(
                    bodies.bob
                )
            ).rejects.toMatchObject(expectedProblem);
        })
    );

    it(
        "create-halight-lightning-bitcoin-herc20-ethereum-erc20-returns-route-not-supported",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob, true))
                .halightLightningBitcoinHerc20EthereumErc20;

            await expect(
                alice.cnd.createHalightLightningBitcoinHerc20EthereumErc20(
                    bodies.alice
                )
            ).rejects.toThrow("Route not yet supported.");
            await expect(
                bob.cnd.createHalightLightningBitcoinHerc20EthereumErc20(
                    bodies.bob
                )
            ).rejects.toThrow("Route not yet supported.");
        })
    );

    it(
        "create-herc20-ethereum-erc20-halight-lightning-bitcoin-returns-invalid-body",
        twoActorTest(async ({ alice }) => {
            await expect(
                alice.cnd.createHerc20EthereumErc20HalightLightningBitcoin(
                    {} as Herc20EthereumErc20HalightLightningBitcoinRequestBody
                )
            ).rejects.toThrow("Invalid body.");
        })
    );

    it(
        "create-halight-lightning-bitcoin-herc20-ethereum-erc20-returns-invalid-body",
        twoActorTest(async ({ alice }) => {
            await expect(
                alice.cnd.createHalightLightningBitcoinHerc20EthereumErc20(
                    {} as HalightLightningBitcoinHerc20EthereumErc20RequestBody
                )
            ).rejects.toThrow("Invalid body.");
        })
    );

    it(
        "get-swap-with-non-existent-id-yields-swap-not-found",
        oneActorTest(async ({ alice }) => {
            try {
                await alice.cnd.fetch(
                    "/swaps/deadbeef-dead-beef-dead-deadbeefdead"
                );
            } catch (error) {
                const expectedProblem = {
                    status: 404,
                    title: "Swap not found.",
                };

                expect(error).toMatchObject(expectedProblem);
            }
        })
    );
});
