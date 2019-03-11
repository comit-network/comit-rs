import { WalletConfig, Wallet } from "./wallet";
import * as chai from "chai";
import {
    Action,
    SwapResponse,
    ComitNodeConfig,
    MetaComitNodeConfig,
} from "./comit";

import chaiHttp = require("chai-http");

chai.use(chaiHttp);

const Toml = require("toml");
const fs = require("fs");
const bitcoin = require("./bitcoin.js");

export interface BtsieveForComitNodeConfig {
    poll_interval_secs: number;
}

export interface TestConfig {
    comit_node: { [key: string]: MetaComitNodeConfig };
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
            this.comitNodeConfig = Toml.parse(
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

    // `chai` used to be passed down. Not sure why?
    poll_comit_node_until(
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
                        setTimeout(() => {
                            this.poll_comit_node_until(
                                location,
                                predicate
                            ).then(result => {
                                final_res(result);
                            });
                        }, 500);
                    }
                });
        });
    }

    do(action: Action) {
        let network = action.payload.network;
        if (network != "regtest") {
            throw Error("Expected network regtest, found " + network);
        }
        let type = action.type;

        switch (type) {
            case "bitcoin-send-amount-to-address": {
                let { to, amount } = action.payload;

                return this.wallet.btc().sendToAddress(to, parseInt(amount));
            }
            case "bitcoin-broadcast-signed-transaction": {
                let { hex } = action.payload;

                return bitcoin.sendRawTransaction(hex);
            }
            case "ethereum-deploy-contract": {
                let { data, amount } = action.payload;

                return this.wallet.eth().deploy_contract(data, amount);
            }
            case "ethereum-invoke-contract": {
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
