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
        ...makeMakerConfig(env, env.ethereum),
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

function makeMakerConfig(
    env: Environment,
    ethereum: EthereumNode
): Pick<NectarConfig, "maker"> {
    const maxBuyQuantity = 0.1;
    const maxSellQuantity = 0.1;
    const feeStrategies = {
        ethereum: {
            service: EthereumGasPriceService.Geth,
            url: ethereum.rpc_url,
        },
    };

    if (env.treasury) {
        return {
            maker: {
                btc_dai: {
                    max_buy_quantity: maxBuyQuantity,
                    max_sell_quantity: maxSellQuantity,
                },
                kraken_api_host: env.treasury.host,
                fee_strategies: feeStrategies,
            },
        };
    } else {
        return {
            maker: {
                btc_dai: {
                    max_sell_quantity: maxSellQuantity,
                },
                fee_strategies: feeStrategies,
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
    btc_dai?: {
        max_buy_quantity?: number;
        max_sell_quantity?: number;
    };
    kraken_api_host?: string;
    fee_strategies?: {
        ethereum?: {
            service: EthereumGasPriceService;
            url: string;
        };
    };
}

enum EthereumGasPriceService {
    Geth = "geth",
    EthGasStation = "eth_gas_station",
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
