import BitcoinRpcClient from "bitcoin-core";
import { readFileAsync, sleep } from "./utils";
import { BitcoinNodeConfig } from "./ledgers";

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
        wallet: "miner",
    });

    // only coins after the first 101 are spendable
    const address = await client.getNewAddress();
    await client.generateToAddress(101, address);

    while (true) {
        await client.generateToAddress(1, address);
        await sleep(1000);
    }
}
