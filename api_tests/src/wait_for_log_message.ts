import { Tail } from "tail";
import { timeout, waitUntilFileExists } from "./utils";

export default async function waitForLogMessage(logFile: string, line: string) {
    await timeout(10000, waitUntilFileExists(logFile));

    // By default tail uses `fs.watch` that watches the inode
    // However, it looks like on Mac OS, the inode get changed at some point
    // To counter that then we use `fs.watchFile` which is actually considered
    // less efficient. Hence only using it on Mac.
    const useWatchFile = process.platform === "darwin" ? true : false;

    const options = {
        fromBeginning: true,
        follow: true,
        useWatchFile,
    };

    const tail = new Tail(logFile, options);

    await timeout(60000, findTextInLog(tail, line));

    tail.unwatch();
}

async function findTextInLog(tail: Tail, text: string) {
    return new Promise((resolve, reject) => {
        tail.on("line", (data: string) => {
            if (data.includes(text)) {
                resolve();
            }
        });

        tail.on("error", (err: any) => {
            reject(err);
        });
    });
}
