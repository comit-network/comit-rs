import { Role, Actors } from "./actors";
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

/*
 * Instantiates a new e2e test based on one actor, Alice.
 */
export function oneActorTest(
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return nActorTest(
        async () =>
            new Actors(new Map([["Alice", await newCndActor("Alice")]])),
        testFn
    );
}

/*
 * Instantiates a new e2e test based on two actors, Alice and Bob.
 */
export function twoActorTest(
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    const alice = newCndActor("Alice");
    const bob = newCndActor("Bob");

    return nActorTest(
        async () =>
            new Actors(
                new Map([
                    ["Alice", await alice],
                    ["Bob", await bob],
                ])
            ),
        testFn
    );
}

/*
 * This test function will take care of instantiating the actors and tearing them down again
 * after the test, regardless if the test succeeded or failed.
 */
function nActorTest(
    makeActors: () => Promise<Actors>,
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return async (done) => {
        const actors = await makeActors();

        try {
            await pTimeout(testFn(actors), 120_000);
        } catch (e) {
            await actors.dumpState();
            throw e;
        } finally {
            await actors.stop();
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

    const actorConfig = await E2ETestActorConfig.for(role);
    const generatedConfig = actorConfig.generateCndConfigFile(ledgerConfig);
    const finalConfig = merge(generatedConfig, global.cndConfigOverrides);
    const cndLogFile = global.getLogFile([testName, `cnd-${role}.log`]);

    logger.info(
        "Created new CndActor with config %s",
        JSON.stringify(finalConfig)
    );

    const cndInstance = new CndInstance(
        global.cargoTargetDir,
        cndLogFile,
        logger,
        finalConfig
    );
    const cndStarting = cndInstance.start();

    const bitcoinWallet = newBitcoinWallet(ledgerConfig, logger);
    const ethereumWallet = newEthereumWallet(
        ledgerConfig,
        global.gethLockDir,
        logger
    );

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
    ethereumLockDir: string,
    logger: Logger
): Promise<EthereumWallet> {
    const ethereumConfig = ledgerConfig.ethereum;
    return ethereumConfig
        ? Web3EthereumWallet.newInstance(
              ethereumConfig.dev_account_key,
              ethereumConfig.rpc_url,
              logger,
              ethereumLockDir,
              ethereumConfig.chain_id
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
