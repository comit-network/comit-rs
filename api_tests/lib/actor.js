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

    poll_comit_node_until(chai, location, state) {
        return new Promise((final_res, rej) => {
            chai.request(this.comit_node_url())
                .get(location)
                .end((err, res) => {
                    if (err) {
                        return rej(err);
                    }
                    res.should.have.status(200);
                    if (res.body.state === state) {
                        final_res(res.body);
                    } else {
                        setTimeout(() => {
                            this.poll_comit_node_until(
                                chai,
                                location,
                                state
                            ).then(result => {
                                final_res(result);
                            });
                        }, 500);
                    }
                });
        });
    }

    async do(action) {
        switch (action.type) {
            case "bitcoin-send-amount-to-address":
                var { to, amount } = action.payload;

                return this.wallet
                    .btc()
                    .send_btc_to_address(to, parseInt(amount));
                break;
            case "bitcoin-broadcast-signed-transaction":
                var { hex } = action.payload;

                return bitcoin_rpc_client.sendRawTransaction(hex);
                break;
            case "ethereum-deploy-contract":
                var { data, amount, gas_limit } = action.payload;

                return this.wallet
                    .eth()
                    .deploy_contract(data, new ethutil.BN(amount, 10));
                break;
            case "ethereum-invoke-contract":
                var {
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
            default:
                return Error("Action type unsupported");
                break;
        }
    }
}

module.exports.create = name => {
    return new Actor(name);
};
