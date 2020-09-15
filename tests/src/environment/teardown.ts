import path from "path";
import killNodes from "./kill_nodes";
import { rimrafAsync } from "./async_fs";

export default async (config: any) => {
    const root = config.rootDir;

    const locksDir = path.join(root, "locks");

    // make sure we don't have any left-over processes
    await killNodes(locksDir);

    await rimrafAsync(locksDir);
};
