import glob from "glob";
import { readFileAsync, rimrafAsync } from "../utils";
import { promisify } from "util";
import processExists from "process-exists";
import path from "path";

const globAsync = promisify(glob);

export default async function killNodes(locksDir: any) {
    const pidFiles = await globAsync("**/*.pid", {
        cwd: locksDir,
    });

    for (const pidFile of pidFiles) {
        const content = await readFileAsync(path.join(locksDir, pidFile), {
            encoding: "utf-8",
        });
        const pid = parseInt(content, 10);

        if (await processExists(pid)) {
            process.stderr.write(
                `Found pid file ${pidFile}, sending SIGINT to process with PID ${pid}\n`
            );
            process.kill(pid, "SIGTERM");
        }
    }

    await rimrafAsync(locksDir);
}
