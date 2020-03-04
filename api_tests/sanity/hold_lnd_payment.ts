import { twoActorTest } from "../lib/actor_test";
import { AssetKind } from "../lib/asset";
import { LedgerKind } from "../lib/ledgers/ledger";
import { expect } from "chai";

setTimeout(function() {
    twoActorTest(
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

    run();
}, 0);
