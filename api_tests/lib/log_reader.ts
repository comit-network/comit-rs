import { Tail } from "tail";

export class LogReader {
    private tail: Tail;

    constructor(logFile: string) {
        const options = { fromBeginning: true, follow: true };

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
