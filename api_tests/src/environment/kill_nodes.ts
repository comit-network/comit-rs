import glob from "glob";
import { promisify } from "util";
import processExists from "process-exists";
import path from "path";
import { promises as asyncFs } from "fs";
import { rimrafAsync } from "./async_fs";

const globAsync = promisify(glob);

export default async function killNodes(locksDir: any) {
    const pidFiles = await globAsync("**/*.pid", {
        cwd: locksDir,
    });

    for (const pidFile of pidFiles) {
        const content = await asyncFs.readFile(path.join(locksDir, pidFile), {
            encoding: "utf-8",
        });
        const pid = parseInt(content, 10);

        if (await processExists(pid)) {
            process.stderr.write(
                `Found pid file ${pidFile}, sending SIGINT to process with PID ${pid}\n`
            );
            try {
                process.kill(pid, "SIGTERM");
            } catch (e) {
                process.stderr.write(`Failed to kill process ${pid}\n`);
            }
        }
    }

    await rimrafAsync(locksDir);
}
