import { ChildProcess, spawn } from "child_process";
import * as path from "path";
import { promises as asyncFs } from "fs";
import getPort from "get-port";
import waitForLogMessage from "./wait_for_log_message";
import { Logger } from "log4js";
import findCacheDir from "find-cache-dir";
import download from "download";
import { platform } from "os";
import { lock } from "proper-lockfile";
import { crashListener } from "./crash_listener";
import { LedgerInstance, LightningNodeConfig } from "./index";
import { waitUntilFileExists } from "./wait_until_file_exists";
import { existsAsync } from "./async_fs";
import { AddressType, createLnRpc } from "@radar/lnrpc";
import { BitcoinFaucet } from "../wallets/bitcoin";

export class LndInstance implements LedgerInstance {
    private process: ChildProcess;

    public static async new(
        dataDir: string,
        logger: Logger,
        bitcoinFaucet: BitcoinFaucet,
        bitcoindDataDir: string,
        pidFile: string
    ) {
        return new LndInstance(
            dataDir,
            logger,
            bitcoinFaucet,
            bitcoindDataDir,
            pidFile,
            await getPort(),
            await getPort(),
            await getPort()
        );
    }

    constructor(
        private readonly dataDir: string,
        private readonly logger: Logger,
        private readonly bitcoinFaucet: BitcoinFaucet,
        private readonly bitcoindDataDir: string,
        private readonly pidFile: string,
        private readonly lndP2pPort: number,
        private readonly lndRpcPort: number,
        private readonly lndRestPort: number
    ) {}

    public async start() {
        await this.createConfigFile();

        await this.execBinary();

        const logFile = this.logPath();

        this.logger.debug("Waiting for lnd password RPC server");
        await waitForLogMessage(logFile, "RPCS: password RPC server listening");

        await this.initWallet();

        this.logger.debug("Waiting for lnd unlocked RPC server");
        await waitForLogMessage(logFile, "RPCS: RPC server listening");

        this.logger.debug(
            "Waiting for admin macaroon file to exist:",
            this.adminMacaroonPath()
        );
        await waitUntilFileExists(this.adminMacaroonPath());
        await this.fundWallet();

        this.logger.debug("Waiting for lightning server to start");
        await waitForLogMessage(logFile, "[INF] BTCN: Server listening on ");

        this.logger.debug("lnd started with PID", this.process.pid);

        await asyncFs.writeFile(this.pidFile, this.process.pid.toString(), {
            encoding: "utf-8",
        });
    }

    private async execBinary() {
        const bin = await this.findBinary("v0.9.1-beta");

        this.logger.debug(`Using binary ${bin}`);
        this.process = spawn(bin, ["--lnddir", this.dataDir], {
            stdio: ["ignore", "ignore", "ignore"], // stdin, stdout, stderr.  These are all logged already.
        });

        this.process.once(
            "exit",
            crashListener(this.process.pid, "lnd", this.logPath())
        );
    }

    private async initWallet() {
        this.logger.debug("Connecting to lnd at", this.grpcSocket);
        const lnRpc = await createLnRpc({
            server: this.grpcSocket,
            tls: this.tlsCertPath(),
        });

        const { cipherSeedMnemonic } = await lnRpc.genSeed({
            seedEntropy: Buffer.alloc(16, this.lndP2pPort),
        });
        const walletPassword = Buffer.from("password", "utf8");
        this.logger.debug(
            "Initialize wallet",
            cipherSeedMnemonic,
            walletPassword
        );
        await lnRpc.initWallet({ cipherSeedMnemonic, walletPassword });

        this.logger.debug("Lnd wallet initialized!");
    }

    private async fundWallet() {
        const lnRpc = await createLnRpc({
            server: this.grpcSocket,
            tls: this.tlsCertPath(),
            macaroonPath: this.adminMacaroonPath(),
        });

        await this.bitcoinFaucet.mint(
            1_000_000_000n, // 10 BTC should be enough money in the LND instance for a few swaps :)
            await lnRpc
                .newAddress({ type: AddressType.NESTED_PUBKEY_HASH })
                .then((r) => r.address),
            async () =>
                lnRpc
                    .walletBalance()
                    .then((r) =>
                        r.confirmedBalance ? BigInt(r.confirmedBalance) : 0n
                    )
        );

        this.logger.debug("Lnd wallet funded with 10 BTC!");
    }

    public isRunning() {
        return this.process != null;
    }

    public logPath() {
        return path.join(this.dataDir, "logs", "bitcoin", "regtest", "lnd.log");
    }

    public tlsCertPath() {
        return path.join(this.dataDir, "tls.cert");
    }

    public adminMacaroonPath() {
        return path.join(
            this.dataDir,
            "data",
            "chain",
            "bitcoin",
            "regtest",
            "admin.macaroon"
        );
    }

    get grpcSocket() {
        return `${this.grpcHost}:${this.grpcPort}`;
    }

    get grpcHost() {
        return "127.0.0.1";
    }

    get grpcPort() {
        return this.lndRpcPort;
    }

    get p2pSocket() {
        return `${this.p2pHost}:${this.p2pPort}`;
    }

    get p2pHost() {
        return "127.0.0.1";
    }

    get p2pPort() {
        return this.lndP2pPort;
    }

    get restPort() {
        return this.lndRestPort;
    }

    get config(): LightningNodeConfig {
        return {
            p2pSocket: this.p2pSocket,
            grpcSocket: this.grpcSocket,
            tlsCertPath: this.tlsCertPath(),
            macaroonPath: this.adminMacaroonPath(),
            restPort: this.restPort,
            dataDir: this.dataDir,
        };
    }

    private async createConfigFile() {
        const output = `[Application Options]
debuglevel=trace

; peer to peer port
listen=127.0.0.1:${this.lndP2pPort}

; gRPC
rpclisten=127.0.0.1:${this.lndRpcPort}

; REST interface
restlisten=127.0.0.1:${this.lndRestPort}

; Do not seek out peers on the network
nobootstrap=true

; Only wait 1 confirmation to open a channel
bitcoin.defaultchanconfs=1

; Allow to have several channels pending at the same time (default is 1)
maxpendingchannels=10

[Bitcoin]

bitcoin.active=true
bitcoin.regtest=true
bitcoin.node=bitcoind

[Bitcoind]

bitcoind.dir=${this.bitcoindDataDir}
`;
        const config = path.join(this.dataDir, "lnd.conf");
        await asyncFs.writeFile(config, output);
    }

    private async findBinary(version: string): Promise<string> {
        const envOverride = process.env.LND_BIN;

        if (envOverride) {
            this.logger.info("Overriding lnd bin with LND_BIN: ", envOverride);

            return envOverride;
        }

        const archiveName = `lnd-${version}`;

        const cacheDir = findCacheDir({
            name: archiveName,
            create: true,
            thunk: true,
        });

        // This path depends on the directory structure inside the archive
        const dirPath = cacheDir(`lnd-${getArch()}-${version}`);
        const binaryPath = path.join(dirPath, "lnd");

        // Recursively create the dir for lock file in case it does not exist
        await asyncFs.mkdir(dirPath, { recursive: true });

        // We do not want both actors to download the lnd binary at the same time
        const lockRelease = await lock(dirPath, {
            lockfilePath: path.join(dirPath, "lock"),
            retries: {
                factor: 1,
                retries: 60 * 5, // Let's give it at least 5min to download (minTimeout * retries = min total wait)
                minTimeout: 1000,
            },
        }).catch(() =>
            Promise.reject(
                new Error(`Failed to acquire lock for downloading lnd`)
            )
        );

        try {
            await existsAsync(binaryPath);
            await lockRelease();
            return binaryPath;
        } catch (e) {
            // Continue and download the file
        }

        const url = downloadUrlFor(version);

        this.logger.info(
            "Binary for version ",
            version,
            " not found at ",
            binaryPath,
            ", downloading from ",
            url
        );

        const destination = cacheDir("");
        await download(url, destination, {
            decompress: true,
            extract: true,
            filename: archiveName,
        });

        this.logger.info("Download completed");
        await lockRelease();
        return binaryPath;
    }
}

function getArch(): string {
    switch (platform()) {
        case "darwin":
            return "darwin-amd64";
        case "linux":
            return "linux-amd64";
        default:
            throw new Error(`Unsupported platform ${platform()}`);
    }
}

function downloadUrlFor(version: string) {
    return `https://github.com/lightningnetwork/lnd/releases/download/${version}/lnd-${getArch()}-${version}.tar.gz`;
}
