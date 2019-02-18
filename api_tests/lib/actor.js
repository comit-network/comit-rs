const Toml = require("toml");
const wallet = require("./wallet.js");
const fs = require("fs");
const bitcoin = require("./bitcoin.js");
const ethutil = require("ethereumjs-util");

const bitcoin_rpc_client = bitcoin.create_client();

class Actor {
    constructor(name) {
        const node_config = global.harness.config.comit_node[name];
        if (!node_config) {
            throw new Error("comit_node." + name + " configuration is needed");
        }
        this.name = name;
        this.host = node_config.host;
        this.config = Toml.parse(
            fs.readFileSync(node_config.config_dir + "/default.toml", "utf8")
        );
        this.wallet = wallet.create(name);
    }

    comit_node_url() {
        return "http://" + this.host + ":" + this.config.http_api.port;
    }

    poll_comit_node_until(chai, location, predicate) {
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
                                chai,
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

    async do(action) {
        let network = action.payload.network;
        if (network != "regtest") {
            throw Error("Expected network regtest, found " + network);
        }
        let type = action.type;

        switch (type) {
            case "bitcoin-send-amount-to-address": {
                let { to, amount } = action.payload;

                return this.wallet
                    .btc()
                    .send_btc_to_address(to, parseInt(amount));
                break;
            }
            case "bitcoin-broadcast-signed-transaction": {
                let { hex } = action.payload;

                return bitcoin_rpc_client.sendRawTransaction(hex);
                break;
            }
            case "ethereum-deploy-contract": {
                let { data, amount, gas_limit } = action.payload;

                return this.wallet.eth().deploy_contract(data, amount);
                break;
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
                    .send_eth_transaction_to(
                        contract_address,
                        data,
                        amount,
                        gas_limit
                    );
                break;
            }
            default:
                throw Error("Action type " + type + " unsupported");
                break;
        }
    }
}

module.exports.create = name => {
    return new Actor(name);
};
