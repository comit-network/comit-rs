import tempfile from "tempfile";
import { BitcoinNodeConfig } from "./bitcoin";
import { EthereumNodeConfig } from "./ethereum";
import { LedgerConfig } from "./ledger_runner";

export interface CndConfigFile {
    http_api: { address: string; port: number };
    database?: { sqlite: string };
    web_gui?: { address: string; port: number };
    network: { listen: string[] };
}

interface BtsieveBitcoin {
    node_url: string;
    network: string;
}

interface BtsieveEthereum {
    node_url: string;
}

export interface BtsieveConfigFile {
    bitcoin?: BtsieveBitcoin;
    ethereum?: BtsieveEthereum;
}

export class E2ETestActorConfig {
    public readonly httpApiPort: number;
    public readonly comitPort: number;
    public readonly seed: Uint8Array;
    public readonly webGuiPort?: number;

    constructor(
        httpApiPort: number,
        comitPort: number,
        seed: string,
        webGuiPort?: number
    ) {
        this.httpApiPort = httpApiPort;
        this.comitPort = comitPort;
        this.seed = new Uint8Array(Buffer.from(seed, "hex"));
        this.webGuiPort = webGuiPort;
    }

    public generateCndConfigFile(
        btsieveConfig: BtsieveConfigFile
    ): CndConfigFile {
        const dbPath = tempfile(".sqlite");
        return {
            http_api: {
                address: "0.0.0.0",
                port: this.httpApiPort,
            },
            database: {
                sqlite: dbPath,
            },
            network: {
                listen: [`/ip4/0.0.0.0/tcp/${this.comitPort}`],
            },
            web_gui: this.webGuiPort
                ? {
                      address: "0.0.0.0",
                      port: this.webGuiPort,
                  }
                : undefined,
            ...btsieveConfig,
        };
    }
}

export const ALICE_CONFIG = new E2ETestActorConfig(
    8000,
    9938,
    "f87165e305b0f7c4824d3806434f9d0909610a25641ab8773cf92a48c9d77670"
);
export const BOB_CONFIG = new E2ETestActorConfig(
    8010,
    9939,
    "1a1707bb54e5fb4deddd19f07adcb4f1e022ca7879e3c8348da8d4fa496ae8e2"
);
export const CHARLIE_CONFIG = new E2ETestActorConfig(
    8020,
    8021,
    "6b49ec1df23d124a16d6a12bd34476579e6e80cdcb97a5438cb76ac5c423c937"
);
// FIXME: David has the same seed as Alice
export const DAVID_CONFIG = new E2ETestActorConfig(
    8123,
    8001,
    "f87165e305b0f7c4824d3806434f9d0909610a25641ab8773cf92a48c9d77670",
    8080
);

export function createBtsieveConfig(
    ledgerConfig: LedgerConfig
): BtsieveConfigFile {
    const config: BtsieveConfigFile = {};

    if (ledgerConfig.bitcoin) {
        config.bitcoin = btsieveBitcoinConfig(ledgerConfig.bitcoin);
    }

    if (ledgerConfig.ethereum) {
        config.ethereum = btsieveEthereumConfig(ledgerConfig.ethereum);
    }

    return config;
}

export function btsieveBitcoinConfig(
    nodeConfig: BitcoinNodeConfig
): BtsieveBitcoin {
    return {
        node_url: `http://${nodeConfig.host}:${nodeConfig.rpcPort}`,
        network: nodeConfig.network,
    };
}

export function btsieveEthereumConfig(
    nodeConfig: EthereumNodeConfig
): BtsieveEthereum {
    return {
        node_url: nodeConfig.rpc_url,
    };
}

export const CND_CONFIGS: {
    [actor: string]: E2ETestActorConfig | undefined;
} = {
    alice: ALICE_CONFIG,
    bob: BOB_CONFIG,
    charlie: CHARLIE_CONFIG,
    david: DAVID_CONFIG,
};
