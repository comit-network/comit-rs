import fs, { promises as asyncFs } from "fs";
import { promisify } from "util";
import { exec } from "child_process";
import rimraf from "rimraf";

export const existsAsync = (filepath: string) =>
    asyncFs.access(filepath, fs.constants.F_OK);
export const openAsync = promisify(fs.open);
export const rimrafAsync = promisify(rimraf);
export const execAsync = promisify(exec);
