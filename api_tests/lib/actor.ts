import { WalletConfig, Wallet } from "./wallet";
import * as chai from "chai";
import {
    Action,
    SwapResponse,
    ComitNodeConfig,
    MetaComitNodeConfig,
} from "./comit";
import * as bitcoin from "./bitcoin";
import * as toml from "toml";
import * as fs from "fs";

import chaiHttp = require("chai-http");
import { sleep } from "./util";
import { MetaBtsieveConfig } from "./btsieve";

chai.use(chaiHttp);

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
            this.comitNodeConfig = toml.parse(
                fs.readFileSync(
                    root +
                        "/" +
                        metaComitNodeConfig.config_dir +
                        "/default.toml",
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

    pollComitNodeUntil(
        location: string,
        predicate: (body: SwapResponse) => boolean
    ) {
        return new Promise((final_res, rej) => {
            chai.request(this.comit_node_url())
                .get(location)
                .end((err, res) => {
                    if (err) {
                        return rej(err);
                    }
                    res.should.have.status(200);
                    let body = Object.assign(
                        { _links: {}, _embedded: {} },
                        res.body
                    );

                    if (predicate(body)) {
                        final_res(body);
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

    async do(action: Action) {
        let network = action.payload.network;
        if (network != "regtest") {
            throw Error("Expected network regtest, found " + network);
        }
        let type = action.type;

        switch (type) {
            case "bitcoin-send-amount-to-address": {
                action.payload.should.include.all.keys("to", "amount");
                let { to, amount } = action.payload;

                return this.wallet.btc().sendToAddress(to, parseInt(amount));
            }
            case "bitcoin-broadcast-signed-transaction": {
                let { hex } = action.payload;

                return bitcoin.sendRawTransaction(hex);
            }
            case "ethereum-deploy-contract": {
                action.payload.should.include.all.keys("data", "amount");
                let { data, amount } = action.payload;

                return this.wallet.eth().deploy_contract(data, amount);
            }
            case "ethereum-invoke-contract": {
                action.payload.should.include.all.keys(
                    "contract_address",
                    "data",
                    "amount",
                    "gas_limit"
                );

                let {
                    contract_address,
                    data,
                    amount,
                    gas_limit,
                } = action.payload;

                return this.wallet
                    .eth()
                    .sendEthTransactionTo(
                        contract_address,
                        data,
                        amount,
                        gas_limit
                    );
            }
            default:
                throw Error("Action type " + type + " unsupported");
        }
    }
}
