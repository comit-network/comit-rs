const BitcoinRpcClient = require("bitcoin-core");
const ethutil = require("ethereumjs-util");
const EthereumTx = require("ethereumjs-tx");
const Toml = require("toml");
const fs = require("fs");
const Web3 = require("web3");
const bitcoin = require("bitcoinjs-lib");

const log4js = require("log4js");
log4js.configure({
    appenders: {
        test_suite: {
            type: "file",
            filename: process.env.LOG_DIR + "/test-suite.log"
        }
    },
    categories: { default: { appenders: ['test_suite'], level: 'ALL' } }
});
const logger = log4js.getLogger( 'test_suite' );

module.exports.logger = function () {
    return logger;
};

// GRPC setup
const grpc = require('grpc');
const protoLoader = require('@grpc/proto-loader');
const options = {
    keepCase: true,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true
};
const packageDefinition = protoLoader.loadSync(process.env.PROJECT_ROOT + '/api_tests/proto/lnd.proto', options);
const lnrpc = grpc.loadPackageDefinition(packageDefinition).lnrpc;
process.env.GRPC_SSL_CIPHER_SUITES = 'HIGH+ECDSA';

function create_lnrpc_client (name, uri) {
    const lnd_cert = fs.readFileSync(process.env.LND_CERTS_DIR + '/' + name + '-tls.cert');
    const ssl_credentials = grpc.credentials.createSsl(lnd_cert);
    const macaroon = grpc.credentials.createFromMetadataGenerator(function (args, callback) {
        const macaroon = fs.readFileSync(process.env.LND_CERTS_DIR + name + '-admin.macaroon').toString('hex');
        let metadata = new grpc.Metadata();
        metadata.add('macaroon', macaroon);
        callback(null, metadata);
    });
    const credentials = grpc.credentials.combineChannelCredentials(ssl_credentials,macaroon);
    return new lnrpc.Lightning(uri, credentials);
};

module.exports.sleep = time => {
    return new Promise((res, rej) => {
        setTimeout(res, time);
    });
};

let _bitcoin_rpc_client;

function bitcoin_rpc_client () {
    return (_bitcoin_rpc_client =
        _bitcoin_rpc_client ||
        new BitcoinRpcClient({
            network: "regtest",
            port: process.env.BITCOIN_RPC_PORT,
            host: process.env.BITCOIN_RPC_HOST,
            username: process.env.BITCOIN_RPC_USERNAME,
            password: process.env.BITCOIN_RPC_PASSWORD
        }));
}

module.exports.bitcoin_rpc_client = () => {
    return bitcoin_rpc_client();
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
    constructor(owner) {
        this.eth_keypair = bitcoin.ECPair.makeRandom({ rng: test_rng });
        this.btc_keypair = bitcoin.ECPair.makeRandom({ rng: test_rng });
        this.bitcoin_utxos = [];
        this.owner = owner;
        this._eth_address = (
            "0x" +
            ethutil
                .privateToAddress(this.eth_keypair.privateKey)
                .toString("hex")
        );
        this._btc_identity = bitcoin.payments.p2wpkh({
            pubkey: this.btc_keypair.publicKey,
            network: regtest
        });

        logger.trace("Generated eth address for %s is %s", this.owner, this._eth_address);
        logger.trace("Generated btc address for %s is %s", this.owner, this._btc_identity.address);
    }

    eth_address() {
        return this._eth_address;
    }

    btc_identity() {
        return this._btc_identity;
    }

    async fund_btc(btc_value) {
        let txid = await bitcoin_rpc_client()
            .sendToAddress(this.btc_identity().address, btc_value);
        let raw_transaction = await bitcoin_rpc_client()
            .getRawTransaction(txid);
        let transaction = bitcoin.Transaction.fromHex(raw_transaction);
        for (let [i, out] of transaction.outs.entries()) {
            if (out.script.equals(this.btc_identity().output)) {
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
        txb.addInput(utxo.txid, utxo.vout, null, this.btc_identity().output);
        //TODO: Add it back to UTXOs after transaction is successful
        txb.addOutput(this.btc_identity().output, change);
        txb.addOutput(bitcoin.address.toOutputScript(to, regtest), value);
        txb.sign(0, key_pair, null, null, input_amount);

        return bitcoin_rpc_client().sendRawTransaction(txb.build().toHex());
    }

    async send_raw_tx(hex) {
        return bitcoin_rpc_client().sendRawTransaction(hex);
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

        logger.trace("Transaction %s transfers %s wei to %s", tx.hash().toString("hex"), tx.value, tx.to.toString("hex"));

        return this.sign_and_send(tx);
    }

    async deploy_erc20_token_contract() {
        return this.deploy_eth_contract(token_contract_deploy);
    }

    async deploy_eth_contract(data = "0x0", value = "0", gas_limit = "0x3D0900") {
        let nonce = await web3.eth.getTransactionCount(this.eth_address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit: gas_limit,
            to: null,
            data: data,
            value: web3.utils.numberToHex(value),
            chainId: 1
        });

        let receipt = await this.sign_and_send(tx);

        let contract_balance = await web3.eth.getBalance(receipt.contractAddress);

        logger.trace("Contract deployed at %s holds %s wei", receipt.contractAddress, contract_balance);

        return receipt;
    }

    async sign_and_send(tx) {
        tx.sign(this.eth_keypair.privateKey);
        const serializedTx = tx.serialize();
        let hex = "0x" + serializedTx.toString("hex");
        let receipt =  await web3.eth.sendSignedTransaction(hex);

        logger.trace("Receipt for transaction %s", receipt.transactionHash, receipt);

        return receipt;
    }
}

class ComitConf {
    constructor(name, should_configure_lnd) {
        this.name = name;
        this.host = process.env[this.name.toUpperCase() + "_COMIT_NODE_HOST"];
        this.config = Toml.parse(
            fs.readFileSync(
                process.env[name.toUpperCase() + "_CONFIG_FILE"],
                "utf8"
            )
        );
        this.wallet = new WalletConf(name);
        if (should_configure_lnd) {
            this.ln = new LightningNetwork(name, this.config.lightning_bitcoin.node_uri);
        }
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

function resolveReject(resolve, reject) {
    return function (err, response) {
        if (err !== null) {
            reject(err);
        }
        else {
            resolve(response);
        }
    };
}

class LightningNetwork {
    constructor(name, uri) {
        this.rpc_client = create_lnrpc_client(name, uri);
        switch (name) {
            case "alice":
                this.host = process.env.lnd_alice_ip;
                break;
            case "bob":
                this.host = process.env.lnd_bob_ip;
                break;
            default:
                throw new Error("Name is needed to setup lnd rpc client");
        }
    }

    async lnNewAddress() {
        const request = {
            type: "np2wkh"
        };
        const ln_client = this.rpc_client;
        return new Promise(function(resolve, reject) {
            ln_client.newAddress(request, resolveReject(resolve, reject));
        });
    }

    async send_btc_to_wallet(btc_value) {
        if (!this._wallet_address) {
            let res = await this.lnNewAddress();
            this._wallet_address = res.address;
        }
        return bitcoin_rpc_client()
            .sendToAddress(this._wallet_address, btc_value);
    }

    async getInfo() {
        const ln_client = this.rpc_client;
        return new Promise(function(resolve, reject) {
            ln_client.getInfo({}, resolveReject(resolve, reject));
        });
    }

    async connectToPeer (to_ln_pubkey, to_ln_host) {
        const from_ln_client = this.rpc_client;
        const address = {
            pubkey: to_ln_pubkey,
            host: to_ln_host
        };
        const request = {
            addr: address,
            perm: false
        };

        return new Promise(function(resolve, reject) {
            from_ln_client.connectPeer(request, resolveReject(resolve, reject));
        });
    }

    async listPeers() {
        const ln_client = this.rpc_client;
        return new Promise(function(resolve, reject) {
            ln_client.listPeers({}, resolveReject(resolve, reject));
        });
    }

    async openChannel (funding_amount_satoshi, to_ln_pubkey) {
        const from_ln_client = this.rpc_client;
        const request = {
            node_pubkey: Buffer.from(to_ln_pubkey, "hex"),
            node_pubkey_string: to_ln_pubkey,
            local_funding_amount: funding_amount_satoshi,
            spend_unconfirmed: true
        };
        let call = from_ln_client.openChannel(request);

        return new Promise(function(resolve, reject) {
            call.on("data", async function (response) {
                if (response.update === "chan_open") {
                    resolve(response);
                } else {
                    await bitcoin_rpc_client().generate(1);;
                }
            });
        });
    }

    async listChannels() {
        const ln_client = this.rpc_client;
        const request = {
            active_only: false,
            inactive_only: false,
            public_only: false,
            private_only: false
        };
        return new Promise(function(resolve, reject) {
            ln_client.listChannels(request, resolveReject(resolve, reject));
        });
    }

    async walletBalance() {
        const ln_client = this.rpc_client;
        return new Promise(function(resolve, reject) {
            ln_client.walletBalance({}, resolveReject(resolve, reject));
        });
    }

    async channelBalance() {
        const ln_client = this.rpc_client;
        return new Promise(function(resolve, reject) {
            ln_client.channelBalance({}, resolveReject(resolve, reject));
        });
    }

    async addInvoice(r_preimage, r_hash, value) {
        const ln_client = this.rpc_client;
        const request = {
            r_preimage: Buffer.from(r_preimage, "hex"),
            r_hash: Buffer.from(r_hash, "hex"),
            value: value
        };
        return new Promise(function(resolve, reject) {
            ln_client.addInvoice(request, resolveReject(resolve, reject));
        });
    }

    async sendPaymentSync(dest, amt, payment_hash, final_cltv_delta) {
        const ln_client = this.rpc_client;
        const request = {
            dest: Buffer.from(dest, "hex"),
            amt,
            payment_hash: Buffer.from(payment_hash, "hex"),
            final_cltv_delta
        };
        return new Promise(function(resolve, reject) {
            ln_client.sendPaymentSync(request, resolveReject(resolve, reject));
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

module.exports.comit_conf = (name, should_configure_lnd) => {
    return new ComitConf(name, should_configure_lnd);
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

        return owner_wallet.send_eth_transaction_to(contract_address, payload, "0x0");
    };
}

module.exports.btc_generate = async function (num = 1) {
    return bitcoin_rpc_client().generate(num);
};

async function btc_balance (address) {
    let btc_balance = await bitcoin_rpc_client().getReceivedByAddress(address);
    return parseFloat(btc_balance) * 100000000;
};

module.exports.btc_balance = async function (address) {
    return btc_balance(address);
};

module.exports.import_address = async function (address) {
    return bitcoin_rpc_client().importAddress(address);
};

async function eth_balance(address) {
    return web3.eth.getBalance(address).then(balance => new ethutil.BN(balance, 10));
}

module.exports.eth_balance = async function (address) {
    return eth_balance(address);
};

module.exports.erc20_balance = async function (token_holder_address, contract_address) {
    const function_identifier = "70a08231";

    const padded_address = token_holder_address
          .replace(/^0x/, "")
          .padStart(64, "0");
    const payload = "0x" + function_identifier + padded_address;

    const tx = {
        from: token_holder_address,
        to: contract_address,
        data: payload
    };

    let hex_balance = await web3.eth.call(tx);
    return web3.utils.toBN(hex_balance);
};

module.exports.log_eth_balance = async function(when, player, address, address_type) {
    logger.info("%s the swap, %s has %s wei at the %s address %s", when, player, await eth_balance(address), address_type, address);
};

module.exports.log_btc_balance = async function(when, player, address, address_type) {
    logger.info("%s the swap, %s has %s satoshis at the %s address %s", when, player, await btc_balance(address), address_type, address);
};
