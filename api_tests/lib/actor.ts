import { Wallet, WalletConfig } from "./wallet";
import { use, request } from "chai";
import { LedgerAction, ComitNodeConfig, MetaComitNodeConfig } from "./comit";
import * as bitcoin from "./bitcoin";
import * as toml from "toml";
import * as fs from "fs";
import { seconds_until, sleep } from "./util";
import { MetaBtsieveConfig } from "./btsieve";
import { Action, Entity } from "../gen/siren";
import chaiHttp = require("chai-http");

use(chaiHttp);

export interface BtsieveForComitNodeConfig {
    poll_interval_secs: number;
}

export interface TestConfig {
    comit_node: { [key: string]: MetaComitNodeConfig };
    btsieve: { [key: string]: MetaBtsieveConfig };
}

export class Actor {
    name: string;
    host: string;
    wallet: Wallet;
    comitNodeConfig: ComitNodeConfig;

    constructor(
        name: string,
        testConfig?: TestConfig,
        root?: string,
        walletConfig?: WalletConfig
    ) {
        this.name = name;
        if (testConfig) {
            const metaComitNodeConfig = testConfig.comit_node[name];
            if (!metaComitNodeConfig) {
                throw new Error("comit_node configuration is needed");
            }

            this.host = metaComitNodeConfig.host;

            const envConfigDir = metaComitNodeConfig.config_dir;
            this.comitNodeConfig = toml.parse(
                fs.readFileSync(
                    root + "/" + envConfigDir + "/default.toml",
                    "utf8"
                )
            );
        }

        if (walletConfig) {
            this.wallet = new Wallet(name, walletConfig);
        }
    }

    comit_node_url() {
        return "http://" + this.host + ":" + this.comitNodeConfig.http_api.port;
    }

    web_gui_url() {
        return "http://" + this.host + ":" + this.comitNodeConfig.web_gui.port;
    }

    async peerId(): Promise<string> {
        let response = await request(this.comit_node_url()).get("/");

        return response.body.id;
    }

    pollComitNodeUntil(
        location: string,
        predicate: (body: Entity) => boolean
    ): Promise<Entity> {
        return new Promise((final_res, rej) => {
            request(this.comit_node_url())
                .get(location)
                .end((err, res) => {
                    if (err) {
                        return rej(err);
                    }
                    res.should.have.status(200);

                    if (predicate(res.body)) {
                        final_res(res.body);
                    } else {
                        setTimeout(async () => {
                            const result = await this.pollComitNodeUntil(
                                location,
                                predicate
                            );
                            final_res(result);
                        }, 500);
                    }
                });
        });
    }

    async doComitAction(action: Action) {
        let { url, body, method } = this.buildRequestFromAction(action);

        let requestFn = request(this.comit_node_url());
        let responsePromise = requestFn(method, url).send(body);

        let response = await responsePromise;

        // We should check against our own content type here to describe "LedgerActions"
        // Don't take it literally but something like `application/vnd.comit-ledger-action+json`
        // For now, checking for `application/json` should do the job as well because accept & decline don't return a body
        if (response.type === "application/json") {
            let body = response.body as LedgerAction;

            return this.doLedgerAction(body);
        } else {
            return Promise.resolve({});
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
                field.class.some((e: string) => e === "feePerByte")
            ) {
                data[field.name] = 20;
            }

            if (
                field.class.some((e: string) => e === "bitcoin") &&
                field.class.some((e: string) => e === "address")
            ) {
                data[field.name] = this.wallet.btc().getNewAddress();
            }
        }

        if (action.type !== "application/json") {
            throw new Error(
                "Warning: only 'application/json' action type is supported, use at your own risk."
            );
        }

        const method = action.method || "GET";
        if (method === "GET") {
            return {
                method,
                url: URI(action.href)
                    .query(URI.buildQuery(data))
                    .toString(),
                body: {},
            };
        } else {
            return {
                method,
                url: action.href,
                body: data,
            };
        }
    }

    async doLedgerAction(action: LedgerAction) {
        let network = action.payload.network;
        if (network != "regtest") {
            throw Error("Expected network regtest, found " + network);
        }
        switch (action.type) {
            case "bitcoin-send-amount-to-address": {
                action.payload.should.include.all.keys("to", "amount");
                let { to, amount } = action.payload;

                return this.wallet.btc().sendToAddress(to, parseInt(amount));
            }
            case "bitcoin-broadcast-signed-transaction": {
                action.payload.should.include.all.keys(
                    "hex",
                    "min_median_block_time"
                );

                let fetchMedianTime = async () => {
                    let blockchainInfo = await bitcoin.getBlockchainInfo();
                    return blockchainInfo.mediantime;
                };

                let { hex, min_median_block_time } = action.payload;

                if (min_median_block_time) {
                    let currentMedianBlockTime = await fetchMedianTime();
                    let diff = min_median_block_time - currentMedianBlockTime;

                    if (diff > 0) {
                        console.log(
                            `Waiting for median time to pass %d`,
                            min_median_block_time
                        );

                        while (diff > 0) {
                            await sleep(1000);

                            currentMedianBlockTime = await fetchMedianTime();
                            diff =
                                min_median_block_time - currentMedianBlockTime;

                            console.log(
                                `Current median time:            %d`,
                                currentMedianBlockTime
                            );
                        }
                    }
                }

                return bitcoin.sendRawTransaction(hex);
            }
            case "ethereum-deploy-contract": {
                action.payload.should.include.all.keys("data", "amount");
                let { data, amount } = action.payload;

                return this.wallet.eth().deploy_contract(data, amount);
            }
            case "ethereum-call-contract": {
                action.payload.should.include.all.keys(
                    "contract_address",
                    "data",
                    "gas_limit",
                    "min_block_timestamp"
                );

                let {
                    contract_address,
                    data,
                    gas_limit,
                    min_block_timestamp,
                } = action.payload;

                if (seconds_until(min_block_timestamp) > 0) {
                    // Ethereum needs a buffer, otherwise the contract code is run but doesn't transfer any funds,
                    // see https://github.com/comit-network/RFCs/issues/62
                    let buffer = 2;
                    let delay = seconds_until(min_block_timestamp) + buffer;

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
