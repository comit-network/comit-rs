import { ChildProcess, spawn } from "child_process";
import waitForLogMessage from "../wait_for_log_message";
import { existsAsync } from "../utils";
import { promises as asyncFs } from "fs";
import getPort from "get-port";
import { Logger } from "log4js";
import { LedgerInstance } from "./index";
import findCacheDir from "find-cache-dir";
import download from "download";
import { platform } from "os";
import chmod from "chmod";
import * as path from "path";

export class ParityInstance implements LedgerInstance {
    private process: ChildProcess;

    public static async new(dataDir: string, pidFile: string, logger: Logger) {
        return new ParityInstance(
            dataDir,
            pidFile,
            logger,
            await getPort({ port: 8545 }),
            await getPort()
        );
    }

    constructor(
        private readonly dataDir: string,
        private readonly pidFile: string,
        private readonly logger: Logger,
        public readonly rpcPort: number,
        public readonly p2pPort: number
    ) {}

    public async start() {
        const bin = await this.findBinary("2.7.2");

        this.logger.info("Using binary", bin);

        await this.createConfigurationFiles();

        this.process = spawn(
            bin,
            [
                `--force-direct`,
                `--no-download`,
                `--base-path=${this.dataDir}`,
                `--config=${this.configTomlPath}`,
                `--chain=${this.chainJsonPath}`,
                `--log-file=${this.logFilePath}`,
                `--password=${this.authorityPwdPath}`,
                `--logging=own_tx=trace,sync=debug,rpc=trace,mining=trace`,
                `--jsonrpc-port=${this.rpcPort}`,
                `--port=${this.p2pPort}`,
                `--no-ws`,
                `--no-ipc`,
                `--unsafe-expose`,
            ],

            {
                cwd: this.dataDir,
                stdio: [
                    "ignore", // stdin
                    "ignore", // stdout
                    "ignore", // stderr
                ],
            }
        );

        this.process.on("exit", (code: number, signal: number) => {
            this.logger.info(
                "parity exited with code",
                code,
                "after signal",
                signal
            );
        });

        await waitForLogMessage(this.logFilePath, "Public node URL:");

        this.logger.info("parity started with PID", this.process.pid);

        await asyncFs.writeFile(this.pidFile, this.process.pid, {
            encoding: "utf-8",
        });
    }

    private async createConfigurationFiles() {
        await ParityInstance.writeFile(this.configTomlPath, CONFIG_TOML);
        await ParityInstance.writeFile(this.authorityJsonPath, AUTHORITY_JSON);
        await ParityInstance.writeFile(this.authorityKeyPath, AUTHORITY_KEY);
        await ParityInstance.writeFile(this.chainJsonPath, CHAIN_JSON);
        await ParityInstance.writeFile(this.authorityPwdPath, AUTHORITY_PWD);
    }

    /**
     * Writes the given string to the given path, creating the necessary directory structure while doing so.
     * @param pathToFile
     * @param content
     */
    private static async writeFile(pathToFile: string, content: string) {
        const { dir } = path.parse(pathToFile);
        await asyncFs.mkdir(dir, { recursive: true });

        await asyncFs.writeFile(pathToFile, content, {
            encoding: "utf-8",
        });
    }

    private get logFilePath() {
        return path.join(this.dataDir, "parity.log");
    }

    private get authorityPwdPath() {
        return path.join(this.dataDir, "authority.pwd");
    }

    private get chainJsonPath() {
        return path.join(this.dataDir, "chain.json");
    }

    private get authorityKeyPath() {
        return path.join(this.dataDir, "network", "key");
    }

    private get authorityJsonPath() {
        return path.join(
            this.dataDir,
            "keys",
            "DevelopmentChain",
            "authority.json"
        );
    }

    private get configTomlPath() {
        return path.join(this.dataDir, "config.toml");
    }

    private async findBinary(version: string): Promise<string> {
        const envOverride = process.env.PARITY_BIN;

        if (envOverride) {
            this.logger.info(
                "Overriding parity bin with PARITY_BIN: ",
                envOverride
            );

            return envOverride;
        }

        const cacheDirPath = `parity-${version}`;
        const binaryName = "parity";

        const cacheDir = findCacheDir({
            name: cacheDirPath,
            create: true,
            thunk: true,
        });
        const binaryPath = cacheDir(binaryName);

        try {
            await existsAsync(binaryPath);
            return binaryPath;
        } catch (e) {
            // Continue and download the file
        }

        const url = downloadUrl(version);

        this.logger.info(
            "Binary for version ",
            version,
            " not found at ",
            binaryPath,
            ", downloading from ",
            url
        );

        await download(url, cacheDir(""), {
            filename: binaryName,
        });

        chmod(binaryPath, {
            execute: true,
        });

        this.logger.info("Download completed");

        return binaryPath;
    }

    public get rpcUrl() {
        return `http://localhost:${this.rpcPort}`;
    }
}

function downloadUrl(version: string) {
    switch (platform()) {
        case "darwin":
            return `https://releases.parity.io/ethereum/v${version}/x86_64-apple-darwin/parity`;
        case "linux":
            return `https://releases.parity.io/ethereum/v${version}/x86_64-unknown-linux-gnu/parity`;
        default:
            throw new Error(`Unsupported platform ${platform()}`);
    }
}

const AUTHORITY_JSON = `{
    "id": "0902d04b-f26e-5c1f-e3ae-78d2c1cb16e7",
    "version": 3,
    "crypto": {
        "cipher": "aes-128-ctr",
        "cipherparams": {
            "iv": "6a829fe7bc656d85f6c2e9fd21784952"
        },
        "ciphertext": "1bfec0b054a648af8fdd0e85662206c65a4af0ed15fede4ad41ca9ab7b504ce2",
        "kdf": "pbkdf2",
        "kdfparams": {
            "c": 10240,
            "dklen": 32,
            "prf": "hmac-sha256",
            "salt": "95f96b5ee22dd537e06076eb8d7078eb7275d29af935782fe476696b11be50e5"
        },
        "mac": "4af2215c3cd9447a5b0512d7d1c3ea5a4435981e1c8f48bf71d7a49c0e5b4986"
    },
    "address": "00bd138abd70e2f00903268f3db08f2d25677c9e",
    "name": "Authority0",
    "meta": "{}"
}`;

const AUTHORITY_KEY =
    "b3244c104fb56d28d3979f6cd14a8b5cf5b109171d293f4454c97c173a9f9374\n";
const AUTHORITY_PWD = "node0";

const CHAIN_JSON = `{
    "name": "DevelopmentChain",
    "engine": {
        "authorityRound": {
            "params": {
                "stepDuration": "1",
                "immediateTransitions": true,
                "maximumEmptySteps": 1000000000,
                "validators": {
                    "list": ["0x00Bd138aBD70e2F00903268F3Db08f2D25677C9e"]
                },
                "maximumUncleCount": 1000000000
            }
        }
    },
    "params": {
        "maximumExtraDataSize": "0x20",
        "minGasLimit": "0x0",
        "networkID": "0x11",
        "gasLimitBoundDivisor": "0x400",
        "eip155Transition": 0,
        "eip140Transition": 0,
        "eip211Transition": 0,
        "eip214Transition": 0,
        "eip658Transition": 0,
        "wasmActivationTransition": 0,
        "eip145Transition": 0,
        "maxTransactionSize": 1000000000,
        "maxCodeSize": 4294967295
    },
    "genesis": {
        "seal": {
            "authorityRound": {
                "step": "0x0",
                "signature": "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
            }
        },
        "difficulty": "0x20000",
        "gasLimit": "0x165A0BC00"
    },
    "accounts": {
        "0x0000000000000000000000000000000000000001": {
            "balance": "1",
            "builtin": {
                "name": "ecrecover",
                "pricing": {
                    "linear": {
                        "base": 3000,
                        "word": 0
                    }
                }
            }
        },
        "0x0000000000000000000000000000000000000002": {
            "balance": "1",
            "builtin": {
                "name": "sha256",
                "pricing": {
                    "linear": {
                        "base": 60,
                        "word": 12
                    }
                }
            }
        },
        "0x0000000000000000000000000000000000000003": {
            "balance": "1",
            "builtin": {
                "name": "ripemd160",
                "pricing": {
                    "linear": {
                        "base": 600,
                        "word": 120
                    }
                }
            }
        },
        "0x0000000000000000000000000000000000000004": {
            "balance": "1",
            "builtin": {
                "name": "identity",
                "pricing": {
                    "linear": {
                        "base": 15,
                        "word": 3
                    }
                }
            }
        },
        "0x0000000000000000000000000000000000000005": {
            "builtin": {
                "name": "modexp",
                "activate_at": 5067000,
                "pricing": {
                    "modexp": {
                        "divisor": 20
                    }
                }
            }
        },
        "0x0000000000000000000000000000000000000006": {
            "builtin": {
                "name": "alt_bn128_add",
                "activate_at": 5067000,
                "pricing": {
                    "linear": {
                        "base": 500,
                        "word": 0
                    }
                }
            }
        },
        "0x0000000000000000000000000000000000000007": {
            "builtin": {
                "name": "alt_bn128_mul",
                "activate_at": 5067000,
                "pricing": {
                    "linear": {
                        "base": 40000,
                        "word": 0
                    }
                }
            }
        },
        "0x00a329c0648769a73afac7f9381e08fb43dbea72": {
            "balance": "1606938044258990275541962092341162602522202993782792835301376"
        }
    }
}`;

const CONFIG_TOML = `
[rpc]
disable = false
interface = "all"
cors = ["all"]
hosts = ["all"]
apis = ["web3", "eth", "net", "parity", "traces", "rpc", "personal", "parity_accounts", "signer", "parity_set"]

[mining]
engine_signer = "0x00Bd138aBD70e2F00903268F3Db08f2D25677C9e"
reseal_on_txs = "none"
reseal_min_period = 1000
reseal_max_period = 5000
gas_floor_target = "0x165A0BC00"
tx_queue_size = 16384
tx_queue_mem_limit = 4096
tx_queue_per_sender = 16384
usd_per_tx = "0"
force_sealing= true
`;
