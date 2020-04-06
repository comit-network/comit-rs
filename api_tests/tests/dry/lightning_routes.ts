/**
 * @logDir lightning_routes
 */

import { oneActorTest } from "../../src/actor_test";
import {
    defaultHalightLightningBitcoinHanEthereumEther,
    defaultHalightLightningBitcoinHerc20EthereumErc20,
    defaultHanEthereumEtherHalightLightningBitcoin,
    defaultHerc20EthereumErc20HalightLightningBitcoin,
} from "../../src/actors/swap_factory";

// ******************************************** //
// Lightning routes                               //
// ******************************************** //

describe("Lightning routes tests", () => {
    it(
        "lightning-routes-post-eth-lnbtc-return-201",
        oneActorTest(async ({ alice }) => {
            const body = defaultHanEthereumEtherHalightLightningBitcoin(
                "",
                {
                    peer_id: "QmXfGiwNESAFWUvDVJ4NLaKYYVopYdV5HbpDSgz5TSypkb",
                },
                "Alice",
                "0x00a329c0648769a73afac7f9381e08fb43dbea72"
            );
            const location = await alice.cnd.createHanEthereumEtherHalightLightningBitcoin(
                body
            );
            expect(typeof location).toBe("string");
        })
    );

    it(
        "lightning-routes-post-erc20-lnbtc-return-400",
        oneActorTest(async ({ alice }) => {
            const promise = alice.cnd.createHerc20EthereumErc20HalightLightningBitcoin(
                defaultHerc20EthereumErc20HalightLightningBitcoin("", {
                    peer_id: "",
                })
            );
            await expect(promise).rejects.toThrow("Route not yet supported");
        })
    );

    it(
        "lightning-routes-post-lnbtc-eth-return-400",
        oneActorTest(async ({ alice }) => {
            const promise = alice.cnd.createHalightLightningBitcoinHanEthereumEther(
                defaultHalightLightningBitcoinHanEthereumEther(
                    "",
                    {
                        peer_id: "",
                    },
                    "0x00a329c0648769a73afac7f9381e08fb43dbea72"
                )
            );
            await expect(promise).rejects.toThrow("Route not yet supported");
        })
    );

    it(
        "lightning-routes-post-lnbtc-erc20-return-400",
        oneActorTest(async ({ alice }) => {
            const promise = alice.cnd.createHalightLightningBitcoinHerc20EthereumErc20(
                defaultHalightLightningBitcoinHerc20EthereumErc20("", {
                    peer_id: "",
                })
            );
            await expect(promise).rejects.toThrow("Route not yet supported");
        })
    );
});
