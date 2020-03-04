/**
 * @ledgers ethereum,bitcoin
 * @logDir e2e
 */

import { twoActorTest } from "../../lib/actor_test";
import { AssetKind } from "../../lib/asset";
import { sleep } from "../../lib/utils";
import { expect } from "chai";
import { LedgerKind } from "../../lib/ledgers/ledger";

// ******************************************** //
// Lightning Sanity Test                        //
// ******************************************** //
describe("E2E: Sanity - LND Alice pays Bob", () => {
    it("sanity-lnd-alice-pays-bob", async function() {
        await twoActorTest("sanity-lnd-alice-pays-bob", async function({
            alice,
            bob,
        }) {
            await alice.sendRequest(
                { ledger: LedgerKind.Lightning, asset: AssetKind.Bitcoin },
                { ledger: LedgerKind.Bitcoin, asset: AssetKind.Bitcoin }
            );
            const { rHash, paymentRequest } = await bob.lnCreateInvoice(
                "20000"
            );
            await alice.lnPayInvoiceWithRequest(paymentRequest);
            await bob.lnAssertInvoiceSettled(rHash);
        });
    });

    it("sanity-lnd-alice-pays-bob-using-hold-invoice", async function() {
        await twoActorTest(
            "sanity-lnd-alice-pays-bob-using-hold-invoice",
            async function({ alice, bob }) {
                await alice.sendRequest(
                    { ledger: LedgerKind.Lightning, asset: AssetKind.Bitcoin },
                    { ledger: LedgerKind.Bitcoin, asset: AssetKind.Bitcoin }
                );

                const satAmount = "10000";
                const finalCltvDelta = 40; // This MUST be 40 to match Bob's invoice. Further investigation needed.

                const { secret, secretHash } = bob.lnCreateSha256Secret();
                await bob.lnCreateHoldInvoice(satAmount, secretHash, 3600);
                const paymentPromise = alice.lnSendPayment(
                    bob,
                    satAmount,
                    secretHash,
                    finalCltvDelta
                );

                await bob.lnSettleInvoice(secret, secretHash);

                const pay = await paymentPromise;
                expect(pay.paymentPreimage.toString("hex")).equals(secret);

                await bob.lnAssertInvoiceSettled(secretHash);
            }
        );
    });
});

// ******************************************** //
// Bitcoin/bitcoin Alpha Ledger/ Alpha Asset    //
// Ethereum/ether Beta Ledger/Beta Asset        //
// ******************************************** //
describe("E2E: Bitcoin/bitcoin - Ethereum/ether", () => {
    it("rfc003-btc-eth-alice-redeems-bob-redeems", async function() {
        await twoActorTest(
            "rfc003-btc-eth-alice-redeems-bob-redeems",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.fund();
                await bob.fund();

                await alice.redeem();
                await bob.redeem();

                await alice.assertSwapped();
                await bob.assertSwapped();
            }
        );
    });

    // ************************ //
    // Refund test              //
    // ************************ //

    it("rfc003-btc-eth-bob-refunds-alice-refunds", async function() {
        await twoActorTest(
            "rfc003-btc-eth-bob-refunds-alice-refunds",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.fund();
                await bob.fund();

                await bob.refund();
                await alice.refund();

                await bob.assertRefunded();
                await alice.assertRefunded();
            }
        );
    });

    it("rfc003-btc-eth-alice-refunds-bob-refunds", async function() {
        await twoActorTest(
            "rfc003-btc-eth-alice-refunds-bob-refunds",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.fund();
                await bob.fund();

                await alice.refund();
                await bob.refund();

                await alice.assertRefunded();
                await bob.assertRefunded();
            }
        );
    });

    // ************************ //
    // Restart cnd test         //
    // ************************ //

    it("rfc003-btc-eth-cnd-can-be-restarted", async function() {
        await twoActorTest(
            "rfc003-btc-eth-cnd-can-be-restarted",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.currentSwapIsAccepted();
                await bob.currentSwapIsAccepted();

                await alice.restart();
                await bob.restart();

                await alice.currentSwapIsAccepted();
                await bob.currentSwapIsAccepted();
            }
        );
    });

    // ************************ //
    // Resume cnd test          //
    // ************************ //

    it("rfc003-btc-eth-resume-alice-down-bob-funds", async function() {
        await twoActorTest(
            "rfc003-btc-eth-resume-alice-down-bob-funds",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.fund();
                alice.stop();

                // Action happens while alice is down.
                await bob.fund();

                // Blocks are geneated every second here, wait to ensure
                // we look into the past for the transaction.
                await sleep(2000);
                await alice.start();

                await alice.redeem();
                await bob.redeem();

                await alice.assertSwapped();
                await bob.assertSwapped();
            }
        );
    });

    it("rfc003-btc-eth-resume-alice-down-bob-redeems", async function() {
        await twoActorTest(
            "rfc003-btc-eth-resume-alice-down-bob-redeems",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.fund();
                await bob.fund();

                await alice.redeem();
                alice.stop();

                // Action happens while alice is down.
                await bob.redeem();

                // Blocks are geneated every second here, wait to ensure
                // we look into the past for the transaction.
                await sleep(2000);
                await alice.start();

                await alice.assertSwapped();
                await bob.assertSwapped();
            }
        );
    });

    it("rfc003-btc-eth-resume-bob-down-alice-funds", async function() {
        await twoActorTest(
            "rfc003-btc-eth-resume-bob-down-alice-funds",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                // Wait for Alice to receive the accept message before stopping Bob's cnd.
                await alice.currentSwapIsAccepted();

                bob.stop();

                // Action happens while bob is down.
                await alice.fund();

                // Blocks are geneated every second here, wait to ensure
                // we look into the past for the transaction.
                await sleep(2000);
                await bob.start();

                await bob.fund();

                await alice.redeem();
                await bob.redeem();

                await alice.assertSwapped();
                await bob.assertSwapped();
            }
        );
    });

    it("rfc003-btc-eth-resume-bob-down-alice-redeems", async function() {
        await twoActorTest(
            "rfc003-btc-eth-resume-bob-down-alice-redeems",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.fund();
                await bob.fund();

                bob.stop();

                // Action happens while bob is down.
                await alice.redeem();

                // Blocks are geneated every second here, wait to ensure
                // we look into the past for the transaction.
                await sleep(2000);
                await bob.start();

                await bob.redeem();

                await alice.assertSwapped();
                await bob.assertSwapped();
            }
        );
    });

    // ************************ //
    // Underfunding test        //
    // ************************ //

    it("rfc003-btc-eth-alice-underfunds-bob-aborts", async function() {
        await twoActorTest(
            "rfc003-btc-eth-alice-underfunds-bob-aborts",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.underfund();

                await bob.assertAlphaIncorrectlyFunded();
                await bob.assertBetaNotDeployed();
                await alice.assertAlphaIncorrectlyFunded();
                await alice.assertBetaNotDeployed();

                await alice.refund();
                await alice.assertRefunded();

                await bob.assertBetaNotDeployed();
            }
        );
    });

    it("rfc003-btc-eth-bob-underfunds-both-refund", async function() {
        await twoActorTest(
            "rfc003-btc-eth-bob-underfunds-both-refund",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();
                await alice.fund();

                await bob.assertAlphaFunded();
                await alice.assertAlphaFunded();

                await bob.underfund();

                await alice.assertBetaIncorrectlyFunded();
                await bob.assertBetaIncorrectlyFunded();

                await bob.refund();
                await bob.assertRefunded();
                await alice.refund();
                await alice.assertRefunded();
            }
        );
    });

    // ************************ //
    // Overfund test            //
    // ************************ //

    it("rfc003-btc-eth-alice-overfunds-bob-aborts", async function() {
        await twoActorTest(
            "rfc003-btc-eth-alice-overfunds-bob-aborts",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();

                await alice.overfund();

                await bob.assertAlphaIncorrectlyFunded();
                await bob.assertBetaNotDeployed();
                await alice.assertAlphaIncorrectlyFunded();
                await alice.assertBetaNotDeployed();

                await alice.refund();
                await alice.assertRefunded();

                await bob.assertBetaNotDeployed();
            }
        );
    });

    it("rfc003-btc-eth-bob-overfunds-both-refund", async function() {
        await twoActorTest(
            "rfc003-btc-eth-bob-overfunds-both-refund",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Ether);
                await bob.accept();
                await alice.fund();

                await bob.assertAlphaFunded();
                await alice.assertAlphaFunded();

                await bob.overfund();

                await alice.assertBetaIncorrectlyFunded();
                await bob.assertBetaIncorrectlyFunded();

                await bob.refund();
                await bob.assertRefunded();
                await alice.refund();
                await alice.assertRefunded();
            }
        );
    });
});

// ******************************************** //
// Ethereum/ether Alpha Ledger/ Alpha Asset     //
// Bitcoin/bitcoin Beta Ledger/Beta Asset       //
// ******************************************** //
describe("E2E: Ethereum/ether - Bitcoin/bitcoin", () => {
    it("rfc003-eth-btc-alice-redeems-bob-redeems", async function() {
        await twoActorTest(
            "rfc003-eth-btc-alice-redeems-bob-redeems",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
                await bob.accept();

                await alice.fund();
                await bob.fund();

                await alice.redeem();
                await bob.redeem();

                await alice.assertSwapped();
                await bob.assertSwapped();
            }
        );
    });

    // ************************ //
    // Ignore Failed ETH TX     //
    // ************************ //

    it("rfc003-eth-btc-alpha-deploy-fails", async function() {
        await twoActorTest("rfc003-eth-btc-alpha-deploy-fails", async function({
            alice,
            bob,
        }) {
            await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
            await bob.accept();

            await alice.fundLowGas("0x1b000");

            await alice.assertAlphaNotDeployed();
            await bob.assertAlphaNotDeployed();
            await bob.assertBetaNotDeployed();
            await alice.assertBetaNotDeployed();
        });
    });

    // ************************ //
    // Refund tests             //
    // ************************ //

    it("rfc003-eth-btc-bob-refunds-alice-refunds", async function() {
        await twoActorTest(
            "rfc003-eth-btc-bob-refunds-alice-refunds",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
                await bob.accept();

                await alice.fund();
                await bob.fund();

                await bob.refund();
                await alice.refund();

                await bob.assertRefunded();
                await alice.assertRefunded();
            }
        );
    });

    // ************************ //
    // Bitcoin High Fees        //
    // ************************ //

    it("rfc003-eth-btc-alice-redeems-with-high-fee", async function() {
        await twoActorTest(
            "rfc003-eth-btc-alice-redeems-with-high-fee",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Ether, AssetKind.Bitcoin);
                await bob.accept();

                await alice.fund();
                await bob.fund();

                const responsePromise = alice.redeemWithHighFee();

                return expect(responsePromise).to.be.rejected;
            }
        );
    });
});

// ******************************************** //
// Bitcoin/bitcoin Alpha Ledger/ Alpha Asset    //
// Ethereum/erc20 Beta Ledger/Beta Asset        //
// ******************************************** //
describe("E2E: Bitcoin/bitcoin - Ethereum/erc20", () => {
    it("rfc003-btc-eth-erc20-alice-redeems-bob-redeems", async function() {
        await twoActorTest(
            "rfc003-btc-eth-erc20-alice-redeems-bob-redeems",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Erc20);
                await bob.accept();

                await alice.fund();
                await bob.deploy();
                await bob.fund();

                await alice.redeem();
                await bob.redeem();

                await alice.assertSwapped();
                await bob.assertSwapped();
            }
        );
    });

    it("rfc003-btc-eth-erc20-bob-refunds-alice-refunds", async function() {
        await twoActorTest(
            "rfc003-btc-eth-erc20-bob-refunds-alice-refunds",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Bitcoin, AssetKind.Erc20);
                await bob.accept();

                await alice.fund();
                await bob.deploy();
                await bob.fund();

                await alice.refund();
                await bob.refund();

                await alice.assertRefunded();
                await bob.assertRefunded();
            }
        );
    });
});

// ******************************************** //
// Ethereum/erc20 Alpha Ledger/ Alpha Asset     //
// Bitcoin/bitcoin Beta Ledger/Beta Asset       //
// ******************************************** //
describe("E2E: Ethereum/erc20 - Bitcoin/bitcoin", () => {
    it("rfc003-eth-erc20_btc-alice-redeems-bob-redeems", async function() {
        await twoActorTest(
            "rfc003-eth-erc20_btc-alice-redeems-bob-redeems",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Erc20, AssetKind.Bitcoin);
                await bob.accept();

                await alice.deploy();
                await alice.fund();
                await bob.fund();

                await alice.redeem();
                await bob.redeem();

                await alice.assertSwapped();
                await bob.assertSwapped();
            }
        );
    });

    it("rfc003-eth-erc20_btc-bob-refunds-alice-refunds", async function() {
        await twoActorTest(
            "rfc003-eth-erc20_btc-bob-refunds-alice-refunds",
            async function({ alice, bob }) {
                await alice.sendRequest(AssetKind.Erc20, AssetKind.Bitcoin);
                await bob.accept();

                await alice.deploy();
                await alice.fund();
                await bob.fund();

                await alice.refund();
                await bob.refund();

                await alice.assertRefunded();
                await bob.assertRefunded();
            }
        );
    });
});
