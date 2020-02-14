import { twoActorTest } from "../lib_sdk/actor_test";

setTimeout(function() {
    twoActorTest("sanity-lnd-alice-pays-bob", async function({ alice, bob }) {
        //
        // Do a bunch of initialisation like we do in `sendRequest` e.g., init/fund wallet
        //

        await alice.startLnd();
        await bob.startLnd();

        await alice.lnd.fund(); // Send to address: bitcoin wallet -> LND wallet

        await alice.lnd.connect(bob.lnd);
        await alice.lnd.openChannel(bob.lnd);

        const invoice = await bob.lnd.addInvoice(alice.lnd); // Parameter might need to be `alice`?
        await alice.lnd.sendPayment(invoice);

        await alice.lnd.assertInvoiceSettled(invoice);
        await bob.lnd.assertInvoiceSettled(invoice);
    });

    run();
}, 0);
