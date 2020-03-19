import { rimrafAsync } from "../utils";
import { EnvGlobal } from "./prepare";

declare var global: EnvGlobal;

export default async () => {
    // delete the locks dir folder to make sure we don't leave old configuration files behind
    await rimrafAsync(global.locksDir);
};
