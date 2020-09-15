import { sleep } from "../utils";
import { existsAsync } from "./async_fs";

export async function waitUntilFileExists(filepath: string) {
    let logFileExists = false;
    do {
        try {
            await existsAsync(filepath);
            logFileExists = true;
        } catch (e) {
            await sleep(500);
        }
    } while (!logFileExists);
}
