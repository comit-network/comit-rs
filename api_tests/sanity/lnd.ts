import { twoActorTest } from "../lib_sdk/actor_test";

setTimeout(function() {
    twoActorTest("sanity-lnd-alice-pays-bob", async function({ alice, bob }) {
        await alice.startLnd();
        await bob.startLnd();

        await alice.fundLnd();

        await alice.connectLnd(bob);
        await alice.openChannel(bob);

        const invoice = await bob.addInvoice(alice);
        await alice.sendPayment(invoice);

        await alice.assertChannelBalanceSender();
        await bob.assertChannelBalanceReceiver();
    });

    run();
}, 0);
