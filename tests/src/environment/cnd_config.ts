import * as tmp from "tmp";
import getPort from "get-port";
import { BitcoinNode, Environment, EthereumNode } from "./index";
import { merge } from "lodash";

export async function newCndConfig(
    env: Environment,
    overrides: Partial<CndConfig>,
): Promise<CndConfig> {
    const httpApiPort = await getPort();
    const comitPort = await getPort();
    const dataDir = tmp.dirSync();
    dataDir.removeCallback(); // Manual cleanup

    const config = {
        http_api: {
            socket: `0.0.0.0:${httpApiPort}`,
        },
        data: {
            dir: dataDir.name,
        },
        network: {
            listen: [`/ip4/0.0.0.0/tcp/${comitPort}`],
        },
        logging: {
            level: "trace",
        },
        ...makeLedgerConfig(env),
    };

    return merge(config, overrides);
}

export interface CndConfig {
    http_api: HttpApi;
    data?: { dir: string };
    network: { listen: string[]; peer_addresses?: string[] };
    logging: { level: string };
    bitcoin?: BitcoinConfig;
    ethereum?: EthereumConfig;
}

export interface HttpApi {
    socket: string;
}

interface LedgerConfig {
    bitcoin?: BitcoinConfig;
    ethereum?: EthereumConfig;
}

interface Geth {
    node_url: string;
}

interface EthereumConfig {
    chain_id: number;
    geth: Geth;
    tokens: Tokens;
}

interface Tokens {
    dai: string;
}

interface Bitcoind {
    node_url: string;
}

interface BitcoinFees {
    strategy: BitcoinFeeStrategy;
    static?: {
        sat_per_vbyte: number;
    };
    cypherblock?: {
        blockchain_endpoint_url?: string;
    };
}

enum BitcoinFeeStrategy {
    Static = "static",
    CypherBlock = "cypherblock",
}

interface BitcoinConfig {
    network: string;
    bitcoind: Bitcoind;
    fees?: BitcoinFees;
}

function makeLedgerConfig(env: Environment) {
    const ledgerConfig: LedgerConfig = {};

    if (env.bitcoin) {
        ledgerConfig.bitcoin = makeBitcoinConfig(env.bitcoin);
    }

    if (env.ethereum) {
        ledgerConfig.ethereum = makeEthereumConfig(env.ethereum);
    }

    return ledgerConfig;
}

function makeBitcoinConfig(bitcoin: BitcoinNode): BitcoinConfig {
    return {
        bitcoind: {
            node_url: bitcoin.rpcUrl,
        },
        network: bitcoin.network,
    };
}

function makeEthereumConfig(ethereum: EthereumNode): EthereumConfig {
    return {
        chain_id: ethereum.chain_id,
        geth: {
            node_url: ethereum.rpc_url,
        },
        tokens: {
            dai: ethereum.tokenContract,
        },
    };
}
