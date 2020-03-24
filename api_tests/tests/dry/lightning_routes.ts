/**
 * @logDir lightning_routes
 */

import { expect } from "chai";
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
            const promise = alice.cnd.createHanEthereumEtherHalightLightningBitcoin(
                defaultHanEthereumEtherHalightLightningBitcoin("", {
                    peer_id: "",
                })
            );
            return expect(promise).to.eventually.be.rejected.then((error) => {
                expect(error).to.have.property(
                    "message",
                    "Request failed with status code 400"
                );
            });
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
            return expect(promise).to.eventually.be.rejected.then((error) => {
                expect(error).to.have.property(
                    "message",
                    "Request failed with status code 400"
                );
            });
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
            return expect(promise).to.eventually.be.rejected.then((error) => {
                expect(error).to.have.property(
                    "message",
                    "Request failed with status code 400"
                );
            });
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
            return expect(promise).to.eventually.be.rejected.then((error) => {
                expect(error).to.have.property(
                    "message",
                    "Request failed with status code 400"
                );
            });
        })
    );
});
