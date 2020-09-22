import { DumpState, Role, Stoppable } from "./actors";
import pTimeout from "p-timeout";
import { HarnessGlobal, Environment } from "./environment";
import { CndActor } from "./actors/cnd_actor";
import { Logger } from "log4js";
import { BitcoindWallet, BitcoinWallet } from "./wallets/bitcoin";
import {
    newBitcoinStubWallet,
    newEthereumStubWallet,
    newLightningStubChannel,
    newLndStubClient,
    Wallets,
} from "./wallets";
import {
    EthereumFaucet,
    EthereumWallet,
    Web3EthereumWallet,
} from "./wallets/ethereum";
import { newCndConfig } from "./environment/cnd_config";
import { CndInstance } from "./environment/cnd_instance";
import ProvidesCallback = jest.ProvidesCallback;

declare var global: HarnessGlobal;

/**
 * Instantiates two CndActors, the first one in the role of Alice and the second one in the role of Bob.
 * @param testFn
 */
export function startAlice(
    testFn: (alice: CndActor) => Promise<void>
): ProvidesCallback {
    return startCndActorTest(["Alice"], async ([alice]) => testFn(alice));
}

/**
 * Instantiates two CndActors, the first one in the role of Alice and the second one in the role of Bob.
 * Starts respective cnd instances.
 * @param testFn
 */
export function startAliceAndBob(
    testFn: ([alice, bob]: CndActor[]) => Promise<void>
): ProvidesCallback {
    return startCndActorTest(["Alice", "Bob"], async ([alice, bob]) =>
        testFn([alice, bob])
    );
}

/**
 * Instantiates two CndActors, the first one in the role of Alice and the second one in the role of Bob.
 * Does not start the cnd instances.
 * @param testFn
 */
export function createAliceAndBob(
    testFn: ([alice, bob]: CndActor[]) => Promise<void>
): ProvidesCallback {
    return createCndActorTest(["Alice", "Bob"], async ([alice, bob]) =>
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
    return startCndActorTest(["Alice", "Bob"], async ([alice, bob]) => {
        await alice.connect(bob);
        return testFn([alice, bob]);
    });
}

/**
 * Instantiates a set of CndActors with the given roles, executes the provided test function and tears the actors down again.
 *
 * This can be used to set up an arbitrary number of nodes by passing any combination of "Alice" or "Bob" within the `roles` array. For example: `cndActorTest(["Alice", "Alice", "Alice", "Bob"], ...)` will give you four nodes, with the first three being in the role of Alice and the fourth one in the role of Bob.
 */
export function startCndActorTest(
    roles: Role[],
    testFn: (actors: CndActor[]) => Promise<void>
): ProvidesCallback {
    return async (done) => {
        const actors = await Promise.all(
            roles.map(async (role: Role) => {
                return newCndActor(role, true);
            })
        );

        await runTest(actors, () => testFn(actors)).then(done);
    };
}

/**
 * Instantiates a set of CndActors with the given roles, executes the provided test function and tears the actors down
 * again. cnd instances are not started.
 *
 * This can be used to set up an arbitrary number of nodes by passing any combination of "Alice" or "Bob" within the
 * `roles` array. For example: `cndActorTest(["Alice", "Alice", "Alice", "Bob"], ...)` will give you four nodes, with
 * the first three being in the role of Alice and the fourth one in the role of Bob.
 */
export function createCndActorTest(
    roles: Role[],
    testFn: (actors: CndActor[]) => Promise<void>
): ProvidesCallback {
    return async (done) => {
        const actors = await Promise.all(
            roles.map(async (role: Role) => {
                return newCndActor(role, false);
            })
        );

        await runTest(actors, () => testFn(actors)).then(done);
    };
}

async function runTest<A extends Iterable<Stoppable & DumpState>>(
    actors: A,
    testFn: () => Promise<void>
) {
    const logger = global.getLogger(["test_environment"]);

    logger.info("All actors created, running test");

    try {
        await pTimeout(testFn(), 120_000);
    } catch (e) {
        logger.error("Test failed", e);
        for (const actor of actors) {
            await actor.dumpState();
        }
        throw e;
    } finally {
        for (const actor of actors) {
            await actor.stop();
        }
    }
}

async function newCndActor(role: Role, startCnd: boolean) {
    const testName = jasmine.currentTestName;
    if (!testName.match(/[A-z0-9\-]+/)) {
        // We use the test name as a file name for the log and hence need to restrict it.
        throw new Error(
            `Testname '${testName}' is invalid. Only A-z, 0-9 and dashes are allowed.`
        );
    }

    const logger = global.getLogger([testName, role]);

    logger.info("Creating new actor in role", role);

    const cndConfig = await newCndConfig(
        role,
        global.environment,
        global.cndConfigOverrides
    );
    const cndLogFile = global.getLogFile([testName, `cnd-${role}.log`]);

    const cndInstance = new CndInstance(
        global.cargoTargetDir,
        cndLogFile,
        logger,
        cndConfig
    );

    let cndStarting;

    if (startCnd) {
        cndStarting = cndInstance.start();
    }

    const bitcoinWallet = newBitcoinWallet(global.environment, logger);
    const ethereumWallet = newEthereumWallet(global.environment, logger);

    // Await all of the Promises that we started. In JS, Promises are eager and hence already started evaluating. This is an attempt to improve the startup performance of an actor.
    const wallets = new Wallets({
        bitcoin: await bitcoinWallet,
        ethereum: await ethereumWallet,
        lightning: newLightningStubChannel(), // Lightning channels are initialized lazily
    });

    if (cndStarting !== undefined) {
        await cndStarting;
    }

    const lndClient = (() => {
        switch (role) {
            case "Alice":
                return global.lndClients.alice;
            case "Bob":
                return global.lndClients.bob;
        }
    })();

    return new CndActor(
        logger,
        cndInstance,
        wallets,
        role,
        lndClient || newLndStubClient()
    );
}

async function newBitcoinWallet(
    env: Environment,
    logger: Logger
): Promise<BitcoinWallet> {
    const bitcoinConfig = env.bitcoin;
    return bitcoinConfig
        ? BitcoindWallet.newInstance(bitcoinConfig, logger)
        : Promise.resolve(newBitcoinStubWallet());
}

async function newEthereumWallet(
    env: Environment,
    logger: Logger
): Promise<EthereumWallet> {
    const ethereumConfig = env.ethereum;
    return ethereumConfig
        ? Web3EthereumWallet.newInstance(
              ethereumConfig.rpc_url,
              logger,
              new EthereumFaucet(
                  ethereumConfig.devAccount,
                  logger,
                  ethereumConfig.rpc_url,
                  ethereumConfig.chain_id
              ),
              ethereumConfig.chain_id
          )
        : Promise.resolve(newEthereumStubWallet());
}
