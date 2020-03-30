/**
 * @logDir lightning_routes
 */

import { oneActorTest } from "../../src/actor_test";
import {
    defaultHalightLightningBitcoinHanEthereumEther,
    defaultHalightLightningBitcoinHerc20EthereumErc20,
    defaultHanEthereumEtherHalightLightningBitcoin,
    defaultHerc20EthereumErc20HalightLightningBitcoin,
} from "../../src/actors/defaults";

// ******************************************** //
// Lightning routes                               //
// ******************************************** //
describe("Lightning routes tests", () => {
    it(
        "lightning-routes-post-eth-lnbtc-return-400",
        oneActorTest(async ({ alice }) => {
            const body = defaultHanEthereumEtherHalightLightningBitcoin(
                "",
                {
                    peer_id: "QmXfGiwNESAFWUvDVJ4NLaKYYVopYdV5HbpDSgz5TSypkb",
                },
                "Alice"
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
                defaultHalightLightningBitcoinHanEthereumEther("", {
                    peer_id: "",
                })
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
