import { mkdirAsync, readFileAsync, rimrafAsync } from "../utils";
import glob from "glob";
import { promisify } from "util";
import path from "path";

const globAsync = promisify(glob);

export default async (config: any) => {
    const root = config.rootDir;

    const logDir = path.join(root, "log");
    const locksDir = path.join(root, "locks");

    const pidFiles = await globAsync("*.pid", {
        cwd: locksDir,
    });

    for (const pidFile of pidFiles) {
        const pid = await readFileAsync(pidFile, {
            encoding: "utf-8",
        }).then(content => parseInt(content, 10));

        process.kill(pid, "SIGTERM");
    }

    await rimrafAsync(logDir);
    await rimrafAsync(locksDir);
    await mkdirAsync(logDir, { recursive: true });
    await mkdirAsync(locksDir, { recursive: true });
};
