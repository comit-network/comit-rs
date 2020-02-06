import { Tail } from "tail";

export class LogReader {
    private tail: Tail;

    constructor(logFile: string) {
        // By default tail uses `fs.watch` that watches the inode
        // However, it looks like on Mac OS, the inode get changed at some point
        // To counter that then we use `fs.watchFile` which is actually considered
        // less efficient. Hence only using it on Mac.
        const useWatchFile = process.platform === "darwin" ? true : false;

        const options = { fromBeginning: true, follow: true, useWatchFile };
        this.tail = new Tail(logFile, options);
    }

    public async waitForLogMessage(line: string) {
        await this.findTextInLog(line);
        this.unwatch();
    }

    private async findTextInLog(text: string) {
        return new Promise((resolve, reject) => {
            this.tail.on("line", (data: string) => {
                if (data.includes(text)) {
                    resolve();
                }
            });

            this.tail.on("error", (err: any) => {
                reject(err);
            });
        });
    }

    private unwatch() {
        this.tail.unwatch();
    }
}
