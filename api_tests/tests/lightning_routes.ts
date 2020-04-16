import { twoActorTest } from "../src/actor_test";
import SwapFactory from "../src/actors/swap_factory";
import {
    HalightLightningBitcoinHanEthereumEtherRequestBody,
    HalightLightningBitcoinHerc20EthereumErc20RequestBody,
    HanEthereumEtherHalightLightningBitcoinRequestBody,
    Herc20EthereumErc20HalightLightningBitcoinRequestBody,
} from "comit-sdk";

// ******************************************** //
// Lightning routes                               //
// ******************************************** //

describe("Lightning routes tests", () => {
    it(
        "create-han-ethereum-ether-halight-lightning-bitcoin-returns-created",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob, true))
                .hanEthereumEtherHalightLightningBitcoin;

            const aliceSwapLocation = await alice.cnd.createHanEthereumEtherHalightLightningBitcoin(
                bodies.alice
            );
            const bobSwapLocation = await bob.cnd.createHanEthereumEtherHalightLightningBitcoin(
                bodies.bob
            );

            expect(bobSwapLocation).toBeTruthy();
            expect(aliceSwapLocation).toBeTruthy();
        })
    );

    it(
        "create-herc20-ethereum-erc20-halight-lightning-bitcoin-returns-route-not-supported",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob, true))
                .herc20EthereumErc20HalightLightningBitcoin;
            await expect(
                alice.cnd.createHerc20EthereumErc20HalightLightningBitcoin(
                    bodies.alice
                )
            ).rejects.toThrow("Route not yet supported.");
            await expect(
                bob.cnd.createHerc20EthereumErc20HalightLightningBitcoin(
                    bodies.bob
                )
            ).rejects.toThrow("Route not yet supported.");
        })
    );

    it(
        "create-halight-lightning-bitcoin-han-ethereum-ether-returns-route-not-supported",
        twoActorTest(async ({ alice, bob }) => {
            const bodies = (await SwapFactory.newSwap(alice, bob, true))
                .halightLightningBitcoinHanEthereumEther;
            await expect(
                alice.cnd.createHalightLightningBitcoinHanEthereumEther(
                    bodies.alice
                )
            ).rejects.toThrow("Route not yet supported.");
            await expect(
                bob.cnd.createHalightLightningBitcoinHanEthereumEther(
                    bodies.bob
                )
            ).rejects.toThrow("Route not yet supported.");
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
        "create-han-ethereum-ether-halight-lightning-bitcoin-returns-invalid-body",
        twoActorTest(async ({ alice }) => {
            await expect(
                alice.cnd.createHanEthereumEtherHalightLightningBitcoin(
                    {} as HanEthereumEtherHalightLightningBitcoinRequestBody
                )
            ).rejects.toThrow("Invalid body.");
        })
    );

    it(
        "create-halight-lightning-bitcoin-han-ethereum-ether-returns-invalid-body",
        twoActorTest(async ({ alice }) => {
            await expect(
                alice.cnd.createHalightLightningBitcoinHanEthereumEther(
                    {} as HalightLightningBitcoinHanEthereumEtherRequestBody
                )
            ).rejects.toThrow("Invalid body.");
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
});
