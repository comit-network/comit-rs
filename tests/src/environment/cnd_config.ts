import * as tmp from "tmp";
import getPort from "get-port";
import { Role } from "../actors";
import { BitcoinNode, EthereumNode, LedgerNodes, LightningNode } from "./index";
import { merge } from "lodash";

export async function newCndConfig(
    role: Role,
    ledgerNodes: LedgerNodes,
    overrides: Partial<CndConfig>
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
            level: "Trace",
        },
        ...makeLedgerConfig(ledgerNodes, role),
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
    lightning?: LightningConfig;
}

export interface HttpApi {
    socket: string;
}

interface LedgerConfig {
    bitcoin?: BitcoinConfig;
    ethereum?: EthereumConfig;
    lightning?: LightningConfig;
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

interface BitcoinConfig {
    network: string;
    bitcoind: Bitcoind;
}

interface Lnd {
    rest_api_url: string;
    dir: string;
}

interface LightningConfig {
    network: string;
    lnd: Lnd;
}

function makeLedgerConfig(ledgerNodes: LedgerNodes, role: Role) {
    const ledgerConfig: LedgerConfig = {};

    if (ledgerNodes.bitcoin) {
        ledgerConfig.bitcoin = makeBitcoinConfig(ledgerNodes.bitcoin);
    }

    if (ledgerNodes.ethereum) {
        ledgerConfig.ethereum = makeEthereumConfig(ledgerNodes.ethereum);
    }

    switch (role) {
        case "Alice": {
            if (ledgerNodes.aliceLnd) {
                ledgerConfig.lightning = makeLightningConfig(
                    ledgerNodes.aliceLnd
                );
            }
            break;
        }
        case "Bob": {
            if (ledgerNodes.bobLnd) {
                ledgerConfig.lightning = makeLightningConfig(
                    ledgerNodes.bobLnd
                );
            }
            break;
        }
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

function makeLightningConfig(lightning: LightningNode): LightningConfig {
    return {
        network: "regtest",
        lnd: {
            rest_api_url: `https://localhost:${lightning.restPort}`,
            dir: lightning.dataDir,
        },
    };
}
