import { promises as asyncFs } from "fs";
import path from "path";
import killNodes from "./kill_nodes";
import { rimrafAsync } from "./async_fs";

export default async (config: any) => {
    const root = config.rootDir;

    const logDir = path.join(root, "log");
    const locksDir = path.join(root, "locks");

    // make sure we have a clean log dir
    await rimrafAsync(logDir);
    await asyncFs.mkdir(logDir, { recursive: true });

    // make sure we don't have any left-over processes
    await killNodes(locksDir);
    await asyncFs.mkdir(locksDir, { recursive: true });

    process.once("SIGINT", () => {
        process.stderr.write("SIGINT caught, cleaning up environment ...\n");

        // tslint:disable-next-line:no-floating-promises cannot await in a signal listener
        killNodes(locksDir).then(() => process.exit(1));
    });
};
