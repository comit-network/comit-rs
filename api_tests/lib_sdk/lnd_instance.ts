import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { E2ETestActorConfig } from "../lib/config";
import { fileExists } from "./utils";
import { HarnessGlobal } from "../lib/util";
import * as path from "path";
import lnService from "ln-service";
import { Logger } from "log4js";
import { LogReader } from "../lib/log_reader";

declare var global: HarnessGlobal;

export class LndInstance {
    private process: ChildProcess;
    private lndDir: string;
    private lndGrpc: any;

    constructor(
        private readonly logger: Logger,
        private readonly logDir: string,
        private readonly actorConfig: E2ETestActorConfig,
        bitcoindDataDir: string
    ) {
        this.lndDir = this.logDir + "/lnd-" + this.actorConfig.name;
        fs.mkdirSync(this.lndDir);

        this.createConfigFile(bitcoindDataDir);
    }

    public async start() {
        const bin = process.env.LND_BIN ? process.env.LND_BIN : "lnd";

        if (global.verbose) {
            console.log(`[${this.actorConfig.name}] using binary ${bin}`);
        }

        this.process = spawn(bin, ["--lnddir", this.lndDir], {
            stdio: ["ignore", "ignore", "ignore"], // stdin, stdout, stderr.  These are all logged already.
        });

        if (global.verbose) {
            console.log(
                `[${this.actorConfig.name}] process spawned LND with PID ${this.process.pid}`
            );
        }

        this.process.on("exit", (code: number, signal: number) => {
            if (global.verbose) {
                console.log(
                    `cnd ${this.actorConfig.name} exited with ${code ||
                        "signal " + signal}`
                );
            }
        });

        this.logger.debug("Waiting for lnd log file to exist:", this.logPath());
        await fileExists(this.logPath());

        this.logger.debug("Waiting for lnd password RPC server");
        await this.logReader().waitForLogMessage(
            "RPCS: password RPC server listening"
        );

        const cert = Buffer.from(
            fs.readFileSync(this.tlsCertPath(), "utf8"),
            "utf8"
        ).toString("base64");

        {
            const { lnd } = lnService.unauthenticatedLndGrpc({
                cert,
                socket: this.getGrpcSocket(),
            });
            const { seed } = await lnService.createSeed({ lnd });
            await lnService.createWallet({ lnd, seed, password: "password" });
        }

        this.logger.debug("Waiting for lnd unlocked RPC server");
        await this.logReader().waitForLogMessage("RPCS: RPC server listening");
        this.logger.debug(
            "Waiting for admin macaroon file to exist:",
            this.adminMacaroonPath()
        );
        await fileExists(this.adminMacaroonPath());
        const macaroon = fs
            .readFileSync(this.adminMacaroonPath())
            .toString("base64");

        const { lnd } = lnService.authenticatedLndGrpc({
            cert,
            macaroon,
            socket: this.getGrpcSocket(),
        });

        this.lndGrpc = lnd;
        this.logger.debug("Waiting for lnd to catch up with blocks");
        await this.logReader().waitForLogMessage(
            "LNWL: Done catching up block hashes"
        );

        const info = await lnService.getWalletInfo({ lnd: this.lndGrpc });
        this.logger.info("Lnd is ready:", info.public_key);
    }

    public stop() {
        this.process.kill("SIGTERM");
        this.process = null;
    }

    public isRunning() {
        return this.process != null;
    }

    public logPath() {
        return path.join(this.lndDir, "logs", "bitcoin", "regtest", "lnd.log");
    }

    public tlsCertPath() {
        return path.join(this.lndDir, "tls.cert");
    }

    public adminMacaroonPath() {
        return path.join(
            this.lndDir,
            "data",
            "chain",
            "bitcoin",
            "regtest",
            "admin.macaroon"
        );
    }

    public getGrpcSocket() {
        return "127.0.0.1:" + this.actorConfig.lndRpcPort;
    }

    private createConfigFile(bitcoindDataDir: string) {
        const output = `[Application Options]

debuglevel=debug

; peer to peer port
listen=127.0.0.1:${this.actorConfig.lndP2pPort}

; gRPC
rpclisten=127.0.0.1:${this.actorConfig.lndRpcPort}

; REST interface
restlisten=127.0.0.1:${this.actorConfig.lndRestPort}

; Do not seek out peers on the network
nobootstrap=true

[Bitcoin]

bitcoin.active=true
bitcoin.regtest=true
bitcoin.node=bitcoind

[Bitcoind]

bitcoind.dir=${bitcoindDataDir}
`;
        const config = this.lndDir + "/lnd.conf";

        fs.writeFileSync(config, output);
    }

    private logReader() {
        return new LogReader(this.logPath());
    }
}
