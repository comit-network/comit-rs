import { Role } from "./actors";
import pTimeout from "p-timeout";
import { HarnessGlobal, LedgerConfig } from "./environment";
import { CndActor } from "./actors/cnd_actor";
import ProvidesCallback = jest.ProvidesCallback;
import { Logger } from "log4js";
import { BitcoindWallet, BitcoinWallet } from "./wallets/bitcoin";
import {
    newBitcoinStubWallet,
    newEthereumStubWallet,
    newLightningStubWallet,
    Wallets,
} from "./wallets";
import { EthereumWallet, Web3EthereumWallet } from "./wallets/ethereum";
import { LightningWallet } from "./wallets/lightning";
import { E2ETestActorConfig } from "./config";
import { merge } from "lodash";
import { CndInstance } from "./environment/cnd_instance";

declare var global: HarnessGlobal;

/**
 * Instantiates two CndActors, the first one in the role of Alice and the second one in the role of Bob.
 * @param testFn
 */
export function startAlice(
    testFn: (alice: CndActor) => Promise<void>
): ProvidesCallback {
    return cndActorTest(["Alice"], async ([alice]) => testFn(alice));
}

/**
 * Instantiates two CndActors, the first one in the role of Alice and the second one in the role of Bob.
 * @param testFn
 */
export function startAliceAndBob(
    testFn: ([alice, bob]: CndActor[]) => Promise<void>
): ProvidesCallback {
    return cndActorTest(["Alice", "Bob"], async ([alice, bob]) =>
        testFn([alice, bob])
    );
}

/**
 * Instantiates two CndActors, the first one in the role of Alice and the second one in the role of Bob.
 *
 * This function also establishes a network connection between the two.
 * @param testFn
 */
export function startConnectedAliceAndBob(
    testFn: ([alice, bob]: CndActor[]) => Promise<void>
): ProvidesCallback {
    return cndActorTest(["Alice", "Bob"], async ([alice, bob]) => {
        await alice.connect(bob);
        return testFn([alice, bob]);
    });
}

/*
 * Instantiates a set of CndActors with the given roles, executes the provided test function and tears the actors down again.
 *
 * This can be used to set up an arbitrary number of nodes by passing any combination of "Alice" or "Bob" within the `roles` array. For example: `cndActorTest(["Alice", "Alice", "Alice", "Bob"], ...)` will give you four nodes, with the first three being in the role of Alice and the fourth one in the role of Bob.
 */
export function cndActorTest(
    roles: Role[],
    testFn: (actors: CndActor[]) => Promise<void>
): ProvidesCallback {
    return async (done) => {
        const actors = await Promise.all(roles.map(newCndActor));
        global
            .getLogger(["test_environment"])
            .info("All actors created, running test");

        try {
            await pTimeout(testFn(actors), 120_000);
        } catch (e) {
            global.getLogger(["test_environment"]).error("Test failed", e);
            for (const actor of this.actors) {
                await actor.dumpState();
            }
            throw e;
        } finally {
            for (const actor of actors) {
                await actor.stop();
            }
        }
        done();
    };
}

async function newCndActor(role: Role) {
    const testName = jasmine.currentTestName;
    if (!testName.match(/[A-z0-9\-]+/)) {
        // We use the test name as a file name for the log and hence need to restrict it.
        throw new Error(
            `Testname '${testName}' is invalid. Only A-z, 0-9 and dashes are allowed.`
        );
    }

    const ledgerConfig = global.ledgerConfigs;
    const logger = global.getLogger([testName, role]);

    logger.info("Creating new actor in role", role);

    const actorConfig = await E2ETestActorConfig.for(role);
    const generatedConfig = actorConfig.generateCndConfigFile(ledgerConfig);
    const finalConfig = merge(generatedConfig, global.cndConfigOverrides);
    const cndLogFile = global.getLogFile([testName, `cnd-${role}.log`]);

    const cndInstance = new CndInstance(
        global.cargoTargetDir,
        cndLogFile,
        logger,
        finalConfig
    );
    const cndStarting = cndInstance.start();

    const bitcoinWallet = newBitcoinWallet(ledgerConfig, logger);
    const ethereumWallet = newEthereumWallet(ledgerConfig, logger);

    // Await all of the Promises that we started. In JS, Promises are eager and hence already started evaluating. This is an attempt to improve the startup performance of an actor.
    const wallets = new Wallets({
        bitcoin: await bitcoinWallet,
        ethereum: await ethereumWallet,
        lightning: newLightningWallet(global.lndWallets, role, logger),
    });
    await cndStarting;

    return new CndActor(logger, cndInstance, wallets, role);
}

async function newBitcoinWallet(
    ledgerConfig: LedgerConfig,
    logger: Logger
): Promise<BitcoinWallet> {
    const bitcoinConfig = ledgerConfig.bitcoin;
    return bitcoinConfig
        ? BitcoindWallet.newInstance(bitcoinConfig, logger)
        : Promise.resolve(newBitcoinStubWallet(logger));
}

async function newEthereumWallet(
    ledgerConfig: LedgerConfig,
    logger: Logger
): Promise<EthereumWallet> {
    const ethereumConfig = ledgerConfig.ethereum;
    return ethereumConfig
        ? Web3EthereumWallet.newInstance(
              ethereumConfig.rpc_url,
              logger,
              ethereumConfig.chain_id,
              ethereumConfig.devAccount
          )
        : Promise.resolve(newEthereumStubWallet(logger));
}

function newLightningWallet(
    lightningWallets: { alice?: LightningWallet; bob?: LightningWallet },
    actor: Role,
    logger: Logger
): LightningWallet {
    switch (actor) {
        case "Alice": {
            return lightningWallets.alice || newLightningStubWallet(logger);
        }
        case "Bob": {
            return lightningWallets.bob || newLightningStubWallet(logger);
        }
    }
}
