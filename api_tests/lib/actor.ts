import { expect, request, use } from "chai";
import chaiHttp = require("chai-http");
import * as fs from "fs";
// @ts-ignore
import multiaddr from "multiaddr";
import { Response } from "superagent";
import * as toml from "toml";
import URI from "urijs";
import { Action, Entity } from "../gen/siren";
import * as bitcoin from "./bitcoin";
import { MetaBtsieveConfig } from "./btsieve";
import { ComitNodeConfig, LedgerAction, MetaComitNodeConfig } from "./comit";
import { seconds_until, sleep } from "./util";
import { Wallet, WalletConfig } from "./wallet";

use(chaiHttp);

export interface BtsieveForComitNodeConfig {
    poll_interval_secs: number;
}

export interface TestConfig {
    comit_node: { [key: string]: MetaComitNodeConfig };
    btsieve: { [key: string]: MetaBtsieveConfig };
}

/// If declined, what is the reason?
interface DeclineConfig {
    reason: string;
}

export interface SirenActionAutofillParams {
    bitcoinFeePerWU: number;
}

const MOVE_CURSOR_UP_ONE_LINE = "\x1b[1A";

export class Actor {
    public name: string;
    public host: string;
    public wallet: Wallet;
    public comitNodeConfig: ComitNodeConfig;
    private readonly _declineConfig?: DeclineConfig;
    private readonly _sirenActionAutofillParams?: SirenActionAutofillParams;

    constructor(
        name: string,
        testConfig?: TestConfig,
        root?: string,
        walletConfig?: WalletConfig,
        declineConfig?: DeclineConfig,
        sirenActionAutofillParams?: SirenActionAutofillParams
    ) {
        this.name = name;
        this._declineConfig = declineConfig;
        this._sirenActionAutofillParams = sirenActionAutofillParams;
        if (testConfig) {
            const metaComitNodeConfig = testConfig.comit_node[name];
            if (!metaComitNodeConfig) {
                throw new Error("comit_node configuration is needed");
            }

            this.host = metaComitNodeConfig.host;

            const configFile = metaComitNodeConfig.config_file;
            this.comitNodeConfig = toml.parse(
                fs.readFileSync(`${root}/${configFile}`, "utf8")
            );
        }

        if (walletConfig) {
            this.wallet = new Wallet(name, walletConfig);
        }
    }

    public comitNodeHttpApiUrl() {
        return "http://" + this.host + ":" + this.comitNodeConfig.http_api.port;
    }

    public comitNodeNetworkListenAddress() {
        const addr = multiaddr(this.comitNodeConfig.network.listen[0]);
        // Need to convert 0.0.0.0 to 127.0.0.1
        return `/${addr.protoNames()[0]}/${this.host}/${addr.protoNames()[1]}/${
            addr.nodeAddress().port
        }`;
    }

    public webGuiUrl() {
        return "http://" + this.host + ":" + this.comitNodeConfig.web_gui.port;
    }

    public async peerId(): Promise<string> {
        const response = await request(this.comitNodeHttpApiUrl()).get("/");

        return response.body.id;
    }

    public async pollComitNodeUntil(
        location: string,
        predicate: (body: Entity) => boolean
    ): Promise<Entity> {
        const response = await request(this.comitNodeHttpApiUrl()).get(
            location
        );

        expect(response).to.have.status(200);

        if (predicate(response.body)) {
            return response.body;
        } else {
            await sleep(500);

            return this.pollComitNodeUntil(location, predicate);
        }
    }

    public doComitAction(action: Action): Promise<Response> {
        const { url, body, method } = this.buildRequestFromAction(action);

        const agent = request(this.comitNodeHttpApiUrl());

        // let's ditch this stupid HTTP library ASAP to avoid this ...
        switch (method) {
            case "GET": {
                return agent.get(url).send(body);
            }
            case "POST": {
                return agent.post(url).send(body);
            }
        }
    }

    public buildRequestFromAction(action: Action) {
        const data: any = {};

        for (const field of action.fields || []) {
            if (
                field.class.some((e: string) => e === "ethereum") &&
                field.class.some((e: string) => e === "address")
            ) {
                data[field.name] = this.wallet.eth().address();
            }

            if (
                field.class.some((e: string) => e === "bitcoin") &&
                field.class.some((e: string) => e === "feePerWU")
            ) {
                expect(field.class).contains(
                    "feePerByte",
                    "API should be backwards compatible"
                );

                data[field.name] = this._sirenActionAutofillParams
                    ? this._sirenActionAutofillParams.bitcoinFeePerWU
                    : 20;
            }

            if (
                field.class.some((e: string) => e === "bitcoin") &&
                field.class.some((e: string) => e === "address")
            ) {
                data[field.name] = this.wallet.btc().getNewAddress();
            }
        }

        if (action.name === "decline" && this._declineConfig) {
            data[action.fields[0].name] = this._declineConfig.reason;
        }

        const method = action.method || "GET";
        if (method === "GET") {
            return {
                method,
                url: new URI(action.href).query(data).toString(),
                body: {},
            };
        } else {
            if (action.type !== "application/json") {
                throw new Error(
                    "Only application/json is supported for non-GET requests."
                );
            }

            return {
                method,
                url: action.href,
                body: data,
            };
        }
    }

    public async doLedgerAction(action: LedgerAction) {
        const network = action.payload.network;
        if (network !== "regtest") {
            throw Error("Expected network regtest, found " + network);
        }
        switch (action.type) {
            case "bitcoin-send-amount-to-address": {
                action.payload.should.include.all.keys("to", "amount");
                const { to, amount } = action.payload;

                return this.wallet
                    .btc()
                    .sendToAddress(to, parseInt(amount, 10));
            }
            case "bitcoin-broadcast-signed-transaction": {
                action.payload.should.include.all.keys("hex");

                const fetchMedianTime = async () => {
                    const blockchainInfo = await bitcoin.getBlockchainInfo();
                    return blockchainInfo.mediantime;
                };

                const { hex, min_median_block_time } = action.payload;

                if (min_median_block_time) {
                    let currentMedianBlockTime = await fetchMedianTime();
                    let diff = min_median_block_time - currentMedianBlockTime;

                    if (diff > 0) {
                        console.log(
                            `Waiting for median time to pass %d`,
                            min_median_block_time
                        );

                        let escapeSequence = "";

                        while (diff > 0) {
                            await sleep(1000);

                            currentMedianBlockTime = await fetchMedianTime();
                            diff =
                                min_median_block_time - currentMedianBlockTime;

                            console.log(
                                `${escapeSequence}Current median time:            %d`,
                                currentMedianBlockTime
                            );
                            escapeSequence = MOVE_CURSOR_UP_ONE_LINE;
                        }
                    }
                }

                return bitcoin.sendRawTransaction(hex);
            }
            case "ethereum-deploy-contract": {
                action.payload.should.include.all.keys("data", "amount");
                const { data, amount } = action.payload;

                return this.wallet.eth().deploy_contract(data, amount);
            }
            case "ethereum-call-contract": {
                action.payload.should.include.all.keys(
                    "contract_address",
                    "data",
                    "gas_limit"
                );

                const {
                    contract_address,
                    data,
                    gas_limit,
                    min_block_timestamp,
                } = action.payload;

                if (
                    min_block_timestamp &&
                    seconds_until(min_block_timestamp) > 0
                ) {
                    // Ethereum needs a buffer, otherwise the contract code is run but doesn't transfer any funds,
                    // see https://github.com/comit-network/RFCs/issues/62
                    const buffer = 2;
                    const delay = seconds_until(min_block_timestamp) + buffer;

                    console.log(
                        `Waiting for %d seconds before action can be executed.`,
                        delay
                    );

                    await sleep(delay * 1000);
                }

                return this.wallet
                    .eth()
                    .sendEthTransactionTo(contract_address, data, 0, gas_limit);
            }
            default:
                throw Error(`Action ${action} is not unsupported`);
        }
    }
}
