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
            level: "trace",
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
    const maxBuyQuantity = 0.1;
    const maxSellQuantity = 0.1;

    if (env.treasury) {
        return {
            maker: {
                btc_dai: {
                    max_buy_quantity: maxBuyQuantity,
                    max_sell_quantity: maxSellQuantity,
                },
                kraken_api_host: env.treasury.host,
            },
        };
    } else {
        return {
            maker: {
                btc_dai: {
                    max_sell_quantity: maxSellQuantity,
                },
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
        fees: {
            strategy: BitcoinFeesStrategy.Static,
        },
    };
}

function makeEthereumConfig(ethereumNode: EthereumNode): Ethereum {
    return {
        chain_id: ethereumNode.chain_id,
        node_url: ethereumNode.rpc_url,
        local_dai_contract_address: ethereumNode.tokenContract,
        gas_price: {
            service: EthereumGasPriceService.Geth,
            url: ethereumNode.rpc_url,
        },
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
}

interface Network {
    listen: string[];
}

interface Data {
    dir?: string;
}

interface Logging {
    level?: "trace" | "debug" | "info" | "warn" | "error";
}

interface Bitcoin {
    network?: "mainnet" | "testnet" | "regtest";
    bitcoind?: {
        node_url: string;
    };
    fees?: {
        strategy?: BitcoinFeesStrategy;
        sat_per_vbyte?: number;
        estimate_mode?: BitcoinFeesEstimateMode;
        max_sat_per_vbyte?: number;
    };
}

enum BitcoinFeesStrategy {
    Static = "static",
    Bitcoind = "bitcoind",
}

enum BitcoinFeesEstimateMode {
    Unset = "unset",
    Conservative = "conservative",
    Economical = "economical",
}

interface Ethereum {
    chain_id?: number;
    node_url?: string;
    local_dai_contract_address?: string;
    gas_price?: {
        service: EthereumGasPriceService;
        url: string;
    };
}

enum EthereumGasPriceService {
    Geth = "geth",
    EthGasStation = "eth_gas_station",
}
