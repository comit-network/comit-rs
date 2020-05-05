import * as tmp from "tmp";
import { LedgerConfig } from "./utils";
import getPort from "get-port";
import {
    LightningNodeConfig,
    BitcoinNodeConfig,
    EthereumNodeConfig,
} from "./ledgers";
import { ActorNames } from "./actors/actor";

export interface CndConfigFile {
    http_api: HttpApi;
    data?: { dir: string };
    network: { listen: string[] };
    logging: { level: string };
}

export interface HttpApi {
    socket: string;
}

export class E2ETestActorConfig {
    public readonly data: string;

    public static async for(name: ActorNames) {
        return new E2ETestActorConfig(await getPort(), await getPort(), name);
    }

    constructor(
        public readonly httpApiPort: number,
        public readonly comitPort: number,
        public readonly name: ActorNames
    ) {
        this.httpApiPort = httpApiPort;
        this.comitPort = comitPort;

        const tmpobj = tmp.dirSync();
        tmpobj.removeCallback(); // Manual cleanup

        this.data = tmpobj.name;
    }

    public generateCndConfigFile(ledgerConfig: LedgerConfig): CndConfigFile {
        return {
            http_api: {
                socket: `0.0.0.0:${this.httpApiPort}`,
            },
            data: {
                dir: this.data,
            },
            network: {
                listen: [`/ip4/0.0.0.0/tcp/${this.comitPort}`],
            },
            logging: {
                level: "Trace",
            },
            ...this.createLedgerConnectors(ledgerConfig),
        };
    }

    private createLedgerConnectors(
        ledgerConfig: LedgerConfig
    ): LedgerConnectors {
        const config: LedgerConnectors = {};

        if (ledgerConfig.bitcoin) {
            config.bitcoin = bitcoinConnector(ledgerConfig.bitcoin);
        }

        if (ledgerConfig.ethereum) {
            config.ethereum = ethereumConnector(ledgerConfig.ethereum);
        }

        switch (this.name) {
            case "alice": {
                if (ledgerConfig.aliceLnd) {
                    config.lightning = lightningConnector(
                        ledgerConfig.aliceLnd
                    );
                }
                break;
            }
            case "bob": {
                if (ledgerConfig.bobLnd) {
                    config.lightning = lightningConnector(ledgerConfig.bobLnd);
                }
                break;
            }
            case "charlie":
                {
                    console.warn(
                        "generating lnd config for charlie is not supported at this stage"
                    );
                }
                break;
        }

        return config;
    }
}

interface LedgerConnectors {
    bitcoin?: BitcoinConnector;
    ethereum?: EthereumConnector;
    lightning?: LightningConnector;
}

interface Parity {
    node_url: string;
}

interface EthereumConnector {
    chain_id: number;
    parity: Parity;
}

interface Bitcoind {
    node_url: string;
}

interface BitcoinConnector {
    network: string;
    bitcoind: Bitcoind;
}

interface Lnd {
    rest_api_url: string;
    dir: string;
}

interface LightningConnector {
    network: string;
    lnd: Lnd;
}

function bitcoinConnector(nodeConfig: BitcoinNodeConfig): BitcoinConnector {
    return {
        bitcoind: {
            node_url: nodeConfig.rpcUrl,
        },
        network: nodeConfig.network,
    };
}

function ethereumConnector(nodeConfig: EthereumNodeConfig): EthereumConnector {
    return {
        chain_id: 17,
        parity: {
            node_url: nodeConfig.rpc_url,
        },
    };
}

function lightningConnector(
    nodeConfig: LightningNodeConfig
): LightningConnector {
    return {
        network: "regtest",
        lnd: {
            rest_api_url: `https://localhost:${nodeConfig.restPort}`,
            dir: nodeConfig.dataDir,
        },
    };
}
