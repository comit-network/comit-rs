/**
 * @ledgers lightning
 */

// ******************************************** //
// Lightning Sanity Test                        //
// ******************************************** //
import { twoActorTest } from "../src/actor_test";
import { LedgerKind } from "../src/ledgers/ledger";
import { AssetKind } from "../src/asset";
import { expect } from "chai";

describe("E2E: Sanity - LND Alice pays Bob", () => {
    it(
        "sanity-lnd-alice-pays-bob",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(
                { ledger: LedgerKind.Lightning, asset: AssetKind.Bitcoin },
                { ledger: LedgerKind.Bitcoin, asset: AssetKind.Bitcoin }
            );
            const { rHash, paymentRequest } = await bob.lnCreateInvoice(
                "20000"
            );
            await alice.lnPayInvoiceWithRequest(paymentRequest);
            await bob.lnAssertInvoiceSettled(rHash);
        })
    );

    it(
        "sanity-lnd-alice-pays-bob-using-hold-invoice",
        twoActorTest(async ({ alice, bob }) => {
            await alice.sendRequest(
                { ledger: LedgerKind.Lightning, asset: AssetKind.Bitcoin },
                { ledger: LedgerKind.Bitcoin, asset: AssetKind.Bitcoin }
            );

            const satAmount = "10000";
            const finalCltvDelta = 10;

            const { secret, secretHash } = bob.lnCreateSha256Secret();
            await bob.lnCreateHoldInvoice(
                satAmount,
                secretHash,
                3600,
                finalCltvDelta
            );
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
        })
    );
});
