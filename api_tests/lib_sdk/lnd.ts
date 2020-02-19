import { ChildProcess, spawn } from "child_process";
import * as fs from "fs";
import { E2ETestActorConfig } from "../lib/config";
import { waitUntilFileExists } from "./utils";
import * as path from "path";
import lnService, {
    AuthenticatedLndGrpc,
    Channel,
    CreateInvoiceResponse,
    GetInvoiceResponse,
    PayResponse,
    Peer,
    WalletInfo,
} from "ln-service";
import { Logger } from "log4js";
import { LogReader } from "../lib/log_reader";
import { mkdirAsync, writeFileAsync } from "./utils";
import getPort from "get-port";

export class Lnd {
    private process: ChildProcess;
    private lndDir: string;
    public authenticatedLndGrpc: AuthenticatedLndGrpc;
    private publicKey?: string;

    constructor(
        private readonly logger: Logger,
        private readonly logDir: string,
        private readonly actorConfig: E2ETestActorConfig,
        private readonly bitcoindDataDir: string
    ) {}

    public async start() {
        const bin = process.env.LND_BIN ? process.env.LND_BIN : "lnd";

        this.logger.debug(`[${this.actorConfig.name}] using binary ${bin}`);

        this.lndDir = path.join(this.logDir, "lnd-" + this.actorConfig.name);
        await mkdirAsync(this.lndDir, "755");
        await this.createConfigFile(this.lndDir);

        this.process = spawn(bin, ["--lnddir", this.lndDir], {
            stdio: ["ignore", "ignore", "ignore"], // stdin, stdout, stderr.  These are all logged already.
        });

        this.logger.debug(
            `[${this.actorConfig.name}] process spawned LND with PID ${this.process.pid}`
        );

        this.process.on("exit", (code: number, signal: number) => {
            this.logger.debug(
                `cnd ${this.actorConfig.name} exited with ${code ||
                    "signal " + signal}`
            );
        });

        this.logger.debug("Waiting for lnd log file to exist:", this.logPath());
        await waitUntilFileExists(this.logPath());

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
        await waitUntilFileExists(this.adminMacaroonPath());
        const macaroon = fs
            .readFileSync(this.adminMacaroonPath())
            .toString("base64");

        const { lnd } = lnService.authenticatedLndGrpc({
            cert,
            macaroon,
            socket: this.getGrpcSocket(),
        });

        this.authenticatedLndGrpc = lnd;
        this.logger.debug("Waiting for lnd to catch up with blocks");
        await this.logReader().waitForLogMessage(
            "LNWL: Done catching up block hashes"
        );

        const info = await this.getWalletInfo();
        this.publicKey = info.public_key;
        this.logger.info("Lnd is ready:", this.publicKey);
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

    public getLightningSocket() {
        return "127.0.0.1:" + this.actorConfig.lndP2pPort;
    }

    public async getWalletInfo(): Promise<WalletInfo> {
        return lnService.getWalletInfo({
            lnd: this.authenticatedLndGrpc,
        });
    }

    public async createChainAddress(): Promise<string> {
        const response = await lnService.createChainAddress({
            format: "np2wpkh",
            lnd: this.authenticatedLndGrpc,
        });
        return response.address;
    }

    public async getChainBalance(): Promise<number> {
        return (
            await lnService.getChainBalance({ lnd: this.authenticatedLndGrpc })
        ).chain_balance;
    }

    public async getChannelBalance(): Promise<number> {
        return (
            await lnService.getChannelBalance({
                lnd: this.authenticatedLndGrpc,
            })
        ).channel_balance;
    }

    public addPeer(peer: Lnd): Promise<void> {
        this.logger.debug(
            `Connecting ${this.publicKey}@${this.getLightningSocket()} to ${
                peer.publicKey
            }@${peer.getLightningSocket()}`
        );
        return lnService.addPeer({
            lnd: this.authenticatedLndGrpc,
            public_key: peer.publicKey,
            socket: peer.getLightningSocket(),
        });
    }

    public async getPeers(): Promise<Peer[]> {
        return (
            await lnService.getPeers({
                lnd: this.authenticatedLndGrpc,
            })
        ).peers;
    }

    public async getChannels(): Promise<Channel[]> {
        return (
            await lnService.getChannels({
                lnd: this.authenticatedLndGrpc,
            })
        ).channels;
    }

    public async openChannel(peer: Lnd, sats: number) {
        this.logger.debug(
            `${this.publicKey} is opening a channel with ${peer.publicKey}; quantity: ${sats}`
        );
        return lnService.openChannel({
            lnd: this.authenticatedLndGrpc,
            partner_public_key: peer.publicKey,
            min_confirmations: 1,
            local_tokens: sats,
        });
    }

    public createInvoice(sats: number): Promise<CreateInvoiceResponse> {
        this.logger.debug(
            `${this.publicKey} is creating an invoice; quantity: ${sats}`
        );
        return lnService.createInvoice({
            lnd: this.authenticatedLndGrpc,
            tokens: sats,
        });
    }

    public pay(invoice: string): Promise<PayResponse> {
        console.log("Paying invoice: %s", invoice);
        return lnService.pay({
            lnd: this.authenticatedLndGrpc,
            request: invoice,
        });
    }

    public async getInvoice(id: string): Promise<GetInvoiceResponse> {
        return lnService.getInvoice({
            lnd: this.authenticatedLndGrpc,
            id,
        });
    }

    private async createConfigFile(lndDir: string) {
        // We don't use REST but want a random port so we don't get used port errors.
        const restPort = await getPort();
        const output = `[Application Options]
debuglevel=debug

; peer to peer port
listen=127.0.0.1:${this.actorConfig.lndP2pPort}

; gRPC
rpclisten=127.0.0.1:${this.actorConfig.lndRpcPort}

; REST interface
restlisten=127.0.0.1:${restPort}

; Do not seek out peers on the network
nobootstrap=true

[Bitcoin]

bitcoin.active=true
bitcoin.regtest=true
bitcoin.node=bitcoind

[Bitcoind]

bitcoind.dir=${this.bitcoindDataDir}
`;
        const config = path.join(lndDir, "lnd.conf");
        await writeFileAsync(config, output);
    }

    private logReader() {
        return new LogReader(this.logPath());
    }
}
