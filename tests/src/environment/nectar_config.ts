import { BitcoinNode, EthereumNode, Environment } from "./index";
import getPort from "get-port";
import * as tmp from "tmp";
import URI from "urijs";

export async function newNectarConfig(env: Environment): Promise<NectarConfig> {
    const comitPort = await getPort();
    const dataDir = tmp.dirSync();
    dataDir.removeCallback(); // Manual cleanup

    return {
        data: {
            dir: dataDir.name,
        },
        network: {
            listen: [`/ip4/0.0.0.0/tcp/${comitPort}`],
        },
        logging: {
            level: "Trace",
        },
        ...makeLedgerConfig(env),
        ...makeMakerConfig(env),
    };
}

function makeLedgerConfig(
    env: Environment
): Pick<NectarConfig, "bitcoin" | "ethereum"> {
    const ledgerConfig: { bitcoin?: Bitcoin; ethereum?: Ethereum } = {};

    if (env.bitcoin) {
        ledgerConfig.bitcoin = makeBitcoinConfig(env.bitcoin);
    }

    if (env.ethereum) {
        ledgerConfig.ethereum = makeEthereumConfig(env.ethereum);
    }

    return ledgerConfig;
}

function makeMakerConfig(env: Environment): Pick<NectarConfig, "maker"> {
    const maxSell = {
        bitcoin: 0.1,
        dai: 2000,
    };

    if (env.treasury) {
        return {
            maker: {
                max_sell: maxSell,
                kraken_api_host: env.treasury.host,
            },
        };
    } else {
        return {
            maker: {
                max_sell: maxSell,
            },
        };
    }
}

function makeBitcoinConfig(bitcoin: BitcoinNode): Bitcoin {
    const parts = URI.parse(bitcoin.rpcUrl);

    return {
        bitcoind: {
            node_url: `http://${bitcoin.username}:${bitcoin.password}@${parts.hostname}:${parts.port}/`,
        },
        network: bitcoin.network,
    };
}

function makeEthereumConfig(ethereum: EthereumNode): Ethereum {
    return {
        chain_id: ethereum.chain_id,
        node_url: ethereum.rpc_url,
        local_dai_contract_address: ethereum.tokenContract,
    };
}

export interface NectarConfig {
    maker?: Maker;
    network?: Network;
    data?: Data;
    logging?: Logging;
    bitcoin?: Bitcoin;
    ethereum?: Ethereum;
}

interface Maker {
    spread?: number;
    max_sell?: {
        bitcoin?: number;
        dai?: number;
    };
    kraken_api_host?: string;
}

interface Network {
    listen: string[];
}

interface Data {
    dir?: string;
}

interface Logging {
    level?: "Trace" | "Debug" | "Info" | "Warn" | "Error";
}

interface Bitcoin {
    network?: "mainnet" | "testnet" | "regtest";
    bitcoind?: {
        node_url: string;
    };
}

interface Ethereum {
    chain_id?: number;
    node_url?: string;
    local_dai_contract_address?: string;
}
