import { configure } from "log4js";
import { HarnessGlobal } from "../lib/util";
import { Actors } from "./actors";
import { Actor } from "./actors/actor";

declare var global: HarnessGlobal;

export async function createActors(logFileName: string): Promise<Actors> {
    const loggerFactory = (whoAmI: string) =>
        configure({
            appenders: {
                file: {
                    type: "file",
                    filename: "log/tests/" + logFileName,
                },
            },
            categories: {
                default: { appenders: ["file"], level: "debug" },
            },
        }).getLogger(whoAmI);

    const alice = await Actor.newInstance(
        loggerFactory,
        "alice",
        global.ledgerConfigs,
        global.projectRoot,
        global.logRoot
    );
    const bob = await Actor.newInstance(
        loggerFactory,
        "bob",
        global.ledgerConfigs,
        global.projectRoot,
        global.logRoot
    );

    const actors = new Actors(
        new Map<string, Actor>([
            ["alice", alice],
            ["bob", bob],
        ])
    );

    alice.actors = actors;
    bob.actors = actors;

    return Promise.resolve(actors);
}
