import { twoActorTest } from "../lib_sdk/actor_test";
import { LedgerKind } from "../lib_sdk/ledger";
import { AssetKind } from "../lib_sdk/asset";
import { expect } from "chai";

setTimeout(function() {
    twoActorTest("sanity-lnd-alice-pays-bob", async function({ alice, bob }) {
        await alice.sendRequestOnLightningRoute(
            LedgerKind.Lightning,
            AssetKind.Bitcoin,
            LedgerKind.Bitcoin,
            AssetKind.Bitcoin
        );

        const alicePeers = await alice.wallets
            .getWalletForLedger("lightning")
            .inner.getPeers();
        expect(alicePeers.length).to.equal(1);

        const bobPeers = await bob.wallets
            .getWalletForLedger("lightning")
            .inner.getPeers();
        expect(bobPeers.length).to.equal(1);

        // const invoice = await bob.lnd.addInvoice(alice.lnd); // Parameter might need to be `alice`?
        // await alice.lnd.sendPayment(invoice);
        //
        // await alice.lnd.assertInvoiceSettled(invoice);
        // await bob.lnd.assertInvoiceSettled(invoice);
    });

    run();
}, 0);
