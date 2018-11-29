const BitcoinRpcClient = require("bitcoin-core");
const ethutil = require("ethereumjs-util");
const EthereumTx = require("ethereumjs-tx");
const Toml = require("toml");
const fs = require("fs");
const Web3 = require("web3");
const bitcoin = require("bitcoinjs-lib");

module.exports.sleep = time => {
    return new Promise((res, rej) => {
        setTimeout(res, time);
    });
};

let bitcoin_rpc_client;

module.exports.bitcoin_rpc_client = () => {
    return (bitcoin_rpc_client =
        bitcoin_rpc_client ||
        new BitcoinRpcClient({
            network: "regtest",
            port: process.env.BITCOIN_RPC_PORT,
            host: process.env.BITCOIN_RPC_HOST,
            username: process.env.BITCOIN_RPC_USERNAME,
            password: process.env.BITCOIN_RPC_PASSWORD
        }));
};

//FIXME: Remove this whenever this change:
// https://github.com/bitcoinjs/bitcoinjs-lib/commit/44a98c0fa6487eaf81500427366787a953ff890d#diff-9e60abeb4e2333a5d2f02de53b4edfac
// Hits npm!
const regtest = {
    messagePrefix: "\x18Bitcoin Signed Message:\n",
    bech32: "bcrt",
    bip32: {
        public: 0x043587cf,
        private: 0x04358394
    },
    pubKeyHash: 0x6f,
    scriptHash: 0xc4,
    wif: 0xef
};

let web3;

module.exports.web3 = () => {
    return (web3 =
        web3 ||
        new Web3(
            new Web3.providers.HttpProvider(process.env.ETHEREUM_NODE_ENDPOINT)
        ));
};

let test_rng_counter = 0;

function test_rng() {
    test_rng_counter++;
    return Buffer.from(("" + test_rng_counter).padStart(32, "0"));
}

const token_contract_deploy =
    "0x" +
    fs
        .readFileSync(
            process.env.PROJECT_ROOT +
                "/application/comit_node/tests/parity_client/erc20_token_contract.asm.hex",
            "utf8"
        )
        .trim();

class WalletConf {
    constructor() {
        this.eth_keypair = bitcoin.ECPair.makeRandom({ rng: test_rng });
        this.btc_keypair = bitcoin.ECPair.makeRandom({ rng: test_rng });
        this.bitcoin_utxos = [];
    }

    eth_address() {
        return (
            "0x" +
            ethutil
                .privateToAddress(this.eth_keypair.privateKey)
                .toString("hex")
        );
    }

    btc_address() {
        return bitcoin.payments.p2wpkh({
            pubkey: this.btc_keypair.publicKey,
            network: regtest
        });
    }

    async fund_btc(btc_value) {
        let txid = await module.exports
            .bitcoin_rpc_client()
            .sendToAddress(this.btc_address().address, btc_value);
        let raw_transaction = await module.exports
            .bitcoin_rpc_client()
            .getRawTransaction(txid);
        let transaction = bitcoin.Transaction.fromHex(raw_transaction);
        for (let [i, out] of transaction.outs.entries()) {
            if (out.script.equals(this.btc_address().output)) {
                out.txid = txid;
                out.vout = i;
                this.bitcoin_utxos.push(out);
            }
        }
    }

    async fund_eth(eth_amount) {
        const parity_dev_account = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
        const parity_dev_password = "";
        const tx = {
            from: parity_dev_account,
            to: this.eth_address(),
            value: web3.utils.numberToHex(
                web3.utils.toWei(eth_amount.toString(), "ether")
            )
        };
        return web3.eth.personal.sendTransaction(tx, parity_dev_password);
    }

    async send_btc_to_address(to, value) {
        const txb = new bitcoin.TransactionBuilder();
        const utxo = this.bitcoin_utxos.shift();
        const to_address = bitcoin.address.fromBech32(to);
        const input_amount = utxo.value;
        const key_pair = this.btc_keypair;
        const fee = 2500;
        const change = input_amount - value - fee;
        txb.addInput(utxo.txid, utxo.vout, null, this.btc_address().output);
        //TODO: Add it back to UTXOs after transaction is successful
        txb.addOutput(this.btc_address().output, change);
        txb.addOutput(bitcoin.address.toOutputScript(to, regtest), value);
        txb.sign(0, key_pair, null, null, input_amount);

        return bitcoin_rpc_client.sendRawTransaction(txb.build().toHex());
    }

    async send_raw_tx(hex) {
        return bitcoin_rpc_client.sendRawTransaction(hex);
    }

    async send_eth_transaction_to(
        to,
        data = "0x0",
        value = "0",
        gas_limit = "0x100000"
    ) {
        if (!to) {
            throw new Error("`to` cannot be null");
        }

        let nonce = await web3.eth.getTransactionCount(this.eth_address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit: gas_limit,
            to: to,
            data: data,
            value: web3.utils.numberToHex(value),
            chainId: 1
        });

        tx.sign(this.eth_keypair.privateKey);
        const serializedTx = tx.serialize();
        let hex = "0x" + serializedTx.toString("hex");
        return web3.eth.sendSignedTransaction(hex);
    }

    async deploy_erc20_token_contract() {
        return this.deploy_eth_contract(token_contract_deploy);
    }

    async deploy_eth_contract(data = "0x0", value = "0x0", gas_limit = "0x3D0900") {
        let nonce = await web3.eth.getTransactionCount(this.eth_address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit: gas_limit,
            to: null,
            data: data,
            value: value,
            chainId: 1
        });

        tx.sign(this.eth_keypair.privateKey);
        const serializedTx = tx.serialize();
        let hex = "0x" + serializedTx.toString("hex");
        return web3.eth.sendSignedTransaction(hex);
    }

    async erc20_balance(contract_address) {
        const function_identifier = "70a08231";

        const address = this.eth_address()
            .replace(/^0x/, "")
            .padStart(64, "0");
        const payload = "0x" + function_identifier + address;

        const tx = {
            from: this.eth_address(),
            to: contract_address,
            data: payload
        };

        let hex_balance = await web3.eth.call(tx);
        return web3.utils.toBN(hex_balance);
    }
}

class ComitConf {
    constructor(name, bitcoin_utxo) {
        this.name = name;
        this.host = process.env[this.name.toUpperCase() + "_COMIT_NODE_HOST"];
        this.config = Toml.parse(
            fs.readFileSync(
                process.env[name.toUpperCase() + "_CONFIG_FILE"],
                "utf8"
            )
        );
        this.wallet = new WalletConf();
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

class LedgerQueryServiceConf {
    constructor(host, port) {
        this.host = host;
        this.port = port;
    }

    url() {
        return "http://" + this.host + ":" + this.port;
    }

    poll_until_matches(chai, query_url) {
        return new Promise((final_res, rej) => {
            chai.request(query_url)
                .get("")
                .end((err, res) => {
                    if (err) {
                        return rej(err);
                    }
                    res.should.have.status(200);
                    if (res.body.matches.length !== 0) {
                        final_res(res.body);
                    } else {
                        setTimeout(() => {
                            this.poll_until_matches(chai, query_url).then(
                                result => {
                                    final_res(result);
                                }
                            );
                        }, 200);
                    }
                });
        });
    }
}

module.exports.comit_conf = (name, utxo) => {
    return new ComitConf(name, utxo);
};

module.exports.wallet_conf = (eth_private_key, utxo) => {
    return new WalletConf(eth_private_key, utxo);
};
module.exports.ledger_query_service_conf = (host, port) => {
    return new LedgerQueryServiceConf(host, port);
};

{
    const function_identifier = "40c10f19";
    module.exports.mint_erc20_tokens = (
        owner_wallet,
        contract_address,
        to_address,
        amount
    ) => {
        to_address = to_address.replace(/^0x/, "").padStart(64, "0");
        amount = web3.utils
            .numberToHex(amount)
            .replace(/^0x/, "")
            .padStart(64, "0");
        const payload = "0x" + function_identifier + to_address + amount;

        return owner_wallet.send_eth_transaction_to(contract_address, payload);
    };
}

module.exports.btc_generate = async function (num = 1) {
    return bitcoin_rpc_client.generate(num);
};

module.exports.btc_balance = async function (address) {
    let btc_balance = await bitcoin_rpc_client.getReceivedByAddress(address);
    return parseFloat(btc_balance) * 100000000;
};

module.exports.import_address = async function (address) {
    return bitcoin_rpc_client.importAddress(address);
};

module.exports.eth_balance = async function (address) {
    return web3.eth.getBalance(address);
};
