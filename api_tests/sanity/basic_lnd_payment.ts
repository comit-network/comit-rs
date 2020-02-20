import { twoActorTest } from "../lib/actor_test";
import { AssetKind } from "../lib/asset";
import { LedgerKind } from "../lib/ledgers/ledger";

setTimeout(function() {
    twoActorTest("sanity-lnd-alice-pays-bob", async function({ alice, bob }) {
        await alice.sendRequest(
            { ledger: LedgerKind.Lightning, asset: AssetKind.Bitcoin },
            { ledger: LedgerKind.Bitcoin, asset: AssetKind.Bitcoin }
        );
        const invoice = await bob.createLnInvoice(20000);
        await alice.payLnInvoice(invoice);
        await bob.assertLnInvoiceSettled(invoice);
    });

    run();
}, 0);
