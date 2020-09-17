import { ChildProcess, spawn } from "child_process";
import waitForLogMessage from "./wait_for_log_message";
import { promises as asyncFs } from "fs";
import getPort from "get-port";
import { Logger } from "log4js";
import findCacheDir from "find-cache-dir";
import download from "download";
import { platform } from "os";
import chmod from "chmod";
import * as path from "path";
import { crashListener } from "./crash_listener";
import { Startable } from "./index";
import { existsAsync, openAsync } from "./async_fs";

export class GethInstance implements Startable {
    private process: ChildProcess;
    // The parameter --networkid does not seem to have an effect for dev chain.
    // Nevertheless we define it so that it is obvious where the chain_id is coming from
    public readonly CHAIN_ID = 1337;

    public static async new(dataDir: string, pidFile: string, logger: Logger) {
        return new GethInstance(
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
        const bin = await this.findBinary("1.9.13-cbc4ac26");

        this.logger.info("Using binary", bin);

        await this.createConfigurationFiles();

        const logFile = await openAsync(this.logFilePath, "w");
        this.process = spawn(
            bin,
            [
                `--dev`,
                `--dev.period=1`, // generates a block every X seconds
                `--datadir=${this.dataDir}`,
                `--networkid=${this.CHAIN_ID}`,
                `--rpc`,
                `--rpcport=${this.rpcPort}`,
                `--port=${this.p2pPort}`,
                `--allow-insecure-unlock`,
                `--unlock=${this.devAccount}`,
                `--password=${this.devAccountPasswordFile}`,
            ],

            {
                cwd: this.dataDir,
                stdio: [
                    "ignore", // stdin
                    logFile,
                    logFile,
                ],
            }
        );

        this.process.once(
            "exit",
            crashListener(this.process.pid, "geth", this.logFilePath)
        );

        await waitForLogMessage(this.logFilePath, "mined potential block");

        this.logger.info("geth started with PID", this.process.pid);

        await asyncFs.writeFile(this.pidFile, this.process.pid.toString(), {
            encoding: "utf-8",
        });
    }

    private async createConfigurationFiles() {
        await GethInstance.writeFile(
            this.devAccountKeyFile,
            this.devAccountKey()
        );
        await GethInstance.writeFile(this.devAccountPasswordFile, "");
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
        return path.join(this.dataDir, "geth.log");
    }

    private get devAccountKeyFile() {
        return path.join(this.dataDir, "keystore", DEV_ACCOUNT_KEY_FILE_NAME);
    }

    public get devAccount() {
        return DEV_ACCOUNT_KEY.address;
    }

    private get devAccountPasswordFile() {
        return path.join(this.dataDir, "password");
    }

    private async findBinary(version: string): Promise<string> {
        const envOverride = process.env.GETH_BIN;

        if (envOverride) {
            this.logger.info(
                "Overriding geth bin with GETH_BIN: ",
                envOverride
            );

            return envOverride;
        }

        const cacheDirPath = `geth-${version}`;
        const archiveName = `geth-${version}`;

        const cacheDir = findCacheDir({
            name: cacheDirPath,
            create: true,
            thunk: true,
        });

        const binaryPath = cacheDir(unpackedFolderName(version), "geth");

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
            decompress: true,
            extract: true,
            filename: archiveName,
            strip: 0,
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

    private devAccountKey() {
        return JSON.stringify(DEV_ACCOUNT_KEY);
    }
}

function downloadUrl(version: string) {
    switch (platform()) {
        case "darwin":
            return `https://gethstore.blob.core.windows.net/builds/geth-darwin-amd64-${version}.tar.gz`;
        case "linux":
            return `https://gethstore.blob.core.windows.net/builds/geth-linux-386-${version}.tar.gz`;
        default:
            throw new Error(`Unsupported platform ${platform()}`);
    }
}

function unpackedFolderName(version: string) {
    switch (platform()) {
        case "darwin":
            return `geth-darwin-amd64-${version}`;
        case "linux":
            return `geth-linux-386-${version}`;
        default:
            throw new Error(`Unsupported platform ${platform()}`);
    }
}

const DEV_ACCOUNT_KEY_FILE_NAME =
    "UTC--2020-04-19T11-50-29.037701000Z--0896f60d2a3f0487f293959a84cf1e9bc2597727";
const DEV_ACCOUNT_KEY = {
    address: "0896f60d2a3f0487f293959a84cf1e9bc2597727",
    crypto: {
        cipher: "aes-128-ctr",
        ciphertext:
            "de7519a821dc32fb760977fa1c65ac42d83e3edc035f14d3f547731487385315",
        cipherparams: { iv: "d9f23be3332ba4002a1253f798169f09" },
        kdf: "scrypt",
        kdfparams: {
            dklen: 32,
            n: 262144,
            p: 1,
            r: 8,
            salt:
                "5d4b597f8d1373291571ba5223333c4c7fbbab5769b89f41aac4b8bf1c31978f",
        },
        mac: "774472998174a8100a4eedf9c75b0bdc994d01029d35a862e1b40464c5e703cc",
    },
    id: "41b02d6e-285f-4bcb-ac4f-187b6dbbd491",
    version: 3,
};
