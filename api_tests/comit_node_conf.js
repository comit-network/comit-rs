const Toml = require("toml");
const test_lib = require("./test_lib.js");
const fs = require("fs");

class ComitConf {
    constructor(name, bitcoin_utxo) {
        const node_config = global.harness.config.comit_node[name];
        if (!node_config) {
            throw new Error("comit_node." + name + " configuration is needed");
        }
        this.name = name;
        this.host = node_config.host;
        this.config = Toml.parse(
            fs.readFileSync(node_config.config_dir + "/default.toml", "utf8")
        );
        this.wallet = test_lib.wallet_conf(name);
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
}

module.exports.create = (name, utxo) => {
    return new ComitConf(name, utxo);
};
