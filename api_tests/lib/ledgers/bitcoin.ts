import BitcoinRpcClient from "bitcoin-core";

export interface BitcoinNodeConfig {
    network: string;
    username: string;
    password: string;
    host: string;
    rpcPort: number;
    p2pPort: number;
    dataDir: string;
}

let bitcoinRpcClient: BitcoinRpcClient;
let bitcoinConfig: BitcoinNodeConfig;

export function init(btcConfig: BitcoinNodeConfig) {
    createBitcoinRpcClient(btcConfig);
}

function createBitcoinRpcClient(btcConfig?: BitcoinNodeConfig) {
    if (!btcConfig && !bitcoinConfig) {
        throw new Error("bitcoin configuration is needed");
    }

    if (!bitcoinRpcClient || btcConfig !== bitcoinConfig) {
        bitcoinRpcClient = new BitcoinRpcClient({
            network: btcConfig.network,
            port: btcConfig.rpcPort,
            host: btcConfig.host,
            username: btcConfig.username,
            password: btcConfig.password,
        });
        bitcoinConfig = btcConfig;
    }
    return bitcoinRpcClient;
}

export async function generate(num: number = 1) {
    const client = createBitcoinRpcClient(bitcoinConfig);

    return client.generateToAddress(num, await client.getNewAddress());
}

export async function ensureFunding() {
    const blockHeight = await createBitcoinRpcClient(
        bitcoinConfig
    ).getBlockCount();
    if (blockHeight < 101) {
        const client = createBitcoinRpcClient(bitcoinConfig);

        await client.generateToAddress(
            101 - blockHeight,
            await client.getNewAddress()
        );
    }
}
