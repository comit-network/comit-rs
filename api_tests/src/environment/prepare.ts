import { mkdirAsync, rimrafAsync } from "../utils";
import path from "path";
import { Global } from "@jest/types";

export interface EnvGlobal extends Global.Global {
    locksDir: string;
}

declare var global: EnvGlobal;

module.exports = async (config: any) => {
    const root = config.rootDir;

    const logDir = path.join(root, "log");
    const locksDir = path.join(root, "locks");

    await rimrafAsync(logDir);
    await rimrafAsync(locksDir);
    await mkdirAsync(logDir, { recursive: true });
    await mkdirAsync(locksDir, { recursive: true });

    global.locksDir = locksDir;
};
