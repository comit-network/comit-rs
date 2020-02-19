import { twoActorTest } from "../lib_sdk/actor_test";
import { LedgerKind } from "../lib_sdk/ledger";
import { AssetKind } from "../lib_sdk/asset";

setTimeout(function() {
    twoActorTest("sanity-lnd-alice-pays-bob", async function({ alice, bob }) {
        await alice.sendRequest(
            { ledger: LedgerKind.Lightning, asset: AssetKind.Bitcoin },
            { ledger: LedgerKind.Bitcoin, asset: AssetKind.Bitcoin }
        );
        const invoice = await bob.wallets.lightning.createInvoice(20000);
        await alice.wallets.lightning.pay(invoice);
        await bob.wallets.lightning.assertInvoiceSettled(invoice);
    });

    run();
}, 0);
