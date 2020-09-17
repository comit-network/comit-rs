import { Tail } from "tail";
import pTimeout from "p-timeout";
import { waitUntilFileExists } from "./wait_until_file_exists";

/*
 * Waits until the file at the given path contains the give message.
 *
 * The line matching the message is returned to the caller.
 */
export default async function waitForLogMessage(
    logFile: string,
    message: string
): Promise<string> {
    await pTimeout(waitUntilFileExists(logFile), 10_000);

    // By default tail uses `fs.watch` that watches the inode
    // However, it looks like on Mac OS, the inode get changed at some point
    // To counter that then we use `fs.watchFile` which is actually considered
    // less efficient. Hence only using it on Mac.
    const useWatchFile = process.platform === "darwin";

    const options = {
        fromBeginning: true,
        follow: true,
        useWatchFile,
    };

    const tail = new Tail(logFile, options);

    const line = await pTimeout(
        findTextInLog(tail, message),
        60000,
        `failed to find message '${message}' in log file '${logFile}'`
    );
    tail.unwatch();

    return line;
}

async function findTextInLog(tail: Tail, text: string): Promise<string> {
    return new Promise((resolve, reject) => {
        tail.on("line", (data: string) => {
            if (data.includes(text)) {
                resolve(data);
            }
        });

        tail.on("error", (err: any) => {
            reject(err);
        });
    });
}
