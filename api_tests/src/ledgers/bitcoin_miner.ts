import { BitcoinNodeConfig } from "./bitcoin";
import BitcoinRpcClient from "bitcoin-core";
import { readFileAsync, sleep } from "../utils";

const configFile = process.argv[2];

// tslint:disable-next-line:no-floating-promises
run(configFile);

async function run(configFile: string) {
    const config: BitcoinNodeConfig = await readFileAsync(configFile, {
        encoding: "utf-8",
    }).then(JSON.parse);

    const client = new BitcoinRpcClient({
        network: "regtest",
        host: "localhost",
        port: config.rpcPort,
        username: config.username,
        password: config.password,
    });

    while (true) {
        await client.generateToAddress(1, await client.getNewAddress());

        await sleep(1000);
    }
}
