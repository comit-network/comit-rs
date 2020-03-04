/**
 * @logDir lightning_routes
 */

import { expect } from "chai";
import { oneActorTest } from "../../lib/actor_test";

// ******************************************** //
// Lightning routes                               //
// ******************************************** //
describe("Lightning routes tests", () => {
    it("lightning-routes-post-eth-lnbtc-return-400", async function() {
        await oneActorTest(
            "lightning-routes-post-eth-lnbtc-return-400",
            async function({ alice }) {
                const promise = alice.cnd.createHanEthereumEtherHalightLightningBitcoin();
                return expect(promise).to.eventually.be.rejected.then(error => {
                    expect(error).to.have.property(
                        "message",
                        "Request failed with status code 400"
                    );
                });
            }
        );
    });

    it("lightning-routes-post-erc20-lnbtc-return-400", async function() {
        await oneActorTest(
            "lightning-routes-post-erc20-lnbtc-return-400",
            async function({ alice }) {
                const promise = alice.cnd.createHerc20EthereumErc20HalightLightningBitcoin();
                return expect(promise).to.eventually.be.rejected.then(error => {
                    expect(error).to.have.property(
                        "message",
                        "Request failed with status code 400"
                    );
                });
            }
        );
    });

    it("lightning-routes-post-lnbtc-eth-return-400", async function() {
        await oneActorTest(
            "lightning-routes-post-lnbtc-eth-return-400",
            async function({ alice }) {
                const promise = alice.cnd.createHalightLightningBitcoinHanEthereumEther();
                return expect(promise).to.eventually.be.rejected.then(error => {
                    expect(error).to.have.property(
                        "message",
                        "Request failed with status code 400"
                    );
                });
            }
        );
    });

    it("lightning-routes-post-lnbtc-erc20-return-400", async function() {
        await oneActorTest(
            "lightning-routes-post-lnbtc-erc20-return-400",
            async function({ alice }) {
                const promise = alice.cnd.createHalightLightningBitcoinHerc20EthereumErc20();
                return expect(promise).to.eventually.be.rejected.then(error => {
                    expect(error).to.have.property(
                        "message",
                        "Request failed with status code 400"
                    );
                });
            }
        );
    });
});
