import { ActorName, Actors } from "./actors";
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
    return nActorTest(["Alice"], testFn);
}

/*
 * Instantiates a new e2e test based on two actors, Alice and Bob.
 */
export function twoActorTest(
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return nActorTest(["Alice", "Bob"], testFn);
}

/*
 * This test function will take care of instantiating the actors and tearing them down again
 * after the test, regardless if the test succeeded or failed.
 */
function nActorTest(
    actorNames: ["Alice"] | ["Alice", "Bob"],
    testFn: (actors: Actors) => Promise<void>
): ProvidesCallback {
    return async (done) => {
        const name = jasmine.currentTestName;
        if (!name.match(/[A-z0-9\-]+/)) {
            // We use the test name as a file name for the log and hence need to restrict it.
            throw new Error(
                `Testname '${name}' is invalid. Only A-z, 0-9 and dashes are allowed.`
            );
        }

        const actors = await createActors(name, actorNames);

        try {
            await pTimeout(testFn(actors), 120_000);
        } catch (e) {
            for (const actorName of actorNames) {
                await actors.getActorByName(actorName).dumpState();
            }
            throw e;
        } finally {
            for (const actorName of actorNames) {
                const actor = actors.getActorByName(actorName);
                await actor.stop();
            }
        }
        done();
    };
}

async function createActors(
    testName: string,
    actorNames: ActorName[]
): Promise<Actors> {
    const actorsMap = new Map<string, CndActor>();

    const listPromises: Promise<CndActor>[] = [];
    for (const name of actorNames) {
        listPromises.push(newCndActor(name, testName));
    }
    const createdActors = await Promise.all(listPromises);
    for (const actor of createdActors) {
        actorsMap.set(actor.name, actor);
    }

    const actors = new Actors(actorsMap);

    for (const name of actorNames) {
        actorsMap.get(name).actors = actors;
    }

    return actors;
}

async function newCndActor(name: ActorName, testName: string) {
    const ledgerConfig = global.ledgerConfigs;
    const logger = global.getLogger([testName, name]);

    const actorConfig = await E2ETestActorConfig.for(name);
    const generatedConfig = actorConfig.generateCndConfigFile(ledgerConfig);
    const finalConfig = merge(generatedConfig, global.cndConfigOverrides);
    const cndLogFile = global.getLogFile([testName, `cnd-${name}.log`]);

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
        lightning: newLightningWallet(global.lndWallets, name, logger),
    });
    await cndStarting;

    return new CndActor(logger, cndInstance, wallets, name);
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
    actor: ActorName,
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
