const BitcoinRpcClient = require('bitcoin-core');
const ethutil = require('ethereumjs-util');
const EthereumTx = require('ethereumjs-tx');
const Toml = require('toml');
const fs = require('fs');
const Web3 = require('web3');
const bitcoin = require('bitcoinjs-lib');

module.exports.sleep = (time) => {
    return new Promise((res, rej) => {
        setTimeout(res, time);
    });
};

let bitcoin_rpc_client;

module.exports.bitcoin_rpc_client = () => {
    return bitcoin_rpc_client = bitcoin_rpc_client || new BitcoinRpcClient({
        network: 'regtest',
        port: process.env.BITCOIN_RPC_PORT,
        host: process.env.BITCOIN_RPC_HOST,
        username: process.env.BITCOIN_RPC_USERNAME,
        password: process.env.BITCOIN_RPC_PASSWORD
    });
};

let web3;

module.exports.web3 = () => {
    return web3 = web3 || new Web3(new Web3.providers.HttpProvider(process.env.ETHEREUM_NODE_ENDPOINT));
};

class PlayerConf {
    constructor(name, bitcoin_utxo) {
        this.name = name;
        this.host = process.env[this.name.toUpperCase() + "_COMIT_NODE_HOST"];
        this.config = Toml.parse(fs.readFileSync(process.env[name.toUpperCase() + "_CONFIG_FILE"], 'utf8'));
        this.bitcoin_utxo = bitcoin_utxo;
    }

    eth_private_key() {
        return Buffer.from(this.config.ethereum.private_key, "hex");
    }

    eth_address() {
        return "0x" + ethutil.privateToAddress(this.eth_private_key()).toString("hex");
    }

    comit_node_url() {
        return "http://" + this.host + ":" + this.config.http_api.port;
    }

    poll_comit_node_until(chai, location, status) {
        return new Promise((final_res, rej) => {
            chai.request(this.comit_node_url()).get(location).end((err, res) => {
                if (err) {
                    return rej(err);
                }
                res.should.have.status(200);
                if (res.body.status === status) {
                    final_res(res.body);
                }
                else {
                    setTimeout(() => {
                        this.poll_comit_node_until(chai, location, status).then((result) => {
                            final_res(result);
                        });
                    }, 3000);
                }
            });
        });
    }

    async send_btc_to_p2wsh_address(to, value) {
        const txb = new bitcoin.TransactionBuilder();
        const utxo = this.bitcoin_utxo;
        const to_address = bitcoin.address.fromBech32(to);
        const input_amount = utxo.value;
        const private_key = bitcoin.ECPair.fromWIF(utxo.private_key);
        const fee = 2500;
        const change = input_amount - value - fee;
        txb.addInput(
            utxo.txid,
            utxo.vout,
            null,
            bitcoin.payments.p2wpkh({pubkey: private_key.publicKey}).output
        );
        //TODO: Generate a new address and send it to there
        txb.addOutput('1cMh228HTCiwS8ZsaakH8A8wze1JR5ZsP', change);
        txb.addOutput(bitcoin.payments.p2wsh({hash: to_address.data}).output, value);
        txb.sign(0, private_key, null, null, input_amount);

        return bitcoin_rpc_client.sendRawTransaction(txb.build().toHex());
    }

    async send_btc_to_p2wpkh_address(to, value) {
        const txb = new bitcoin.TransactionBuilder();
        const utxo = this.bitcoin_utxo;
        const to_address = bitcoin.address.fromBech32(to);
        const input_amount = utxo.value;
        const private_key = bitcoin.ECPair.fromWIF(utxo.private_key);
        const fee = 2500;
        const change = input_amount - value - fee;
        txb.addInput(
            utxo.txid,
            utxo.vout,
            null,
            bitcoin.payments.p2wpkh({pubkey: private_key.publicKey}).output
        );
        //TODO: Generate a new address and send it to there
        txb.addOutput('1cMh228HTCiwS8ZsaakH8A8wze1JR5ZsP', change);
        txb.addOutput(bitcoin.payments.p2wpkh({hash: to_address.data}).output, value);
        txb.sign(0, private_key, null, null, input_amount);

        return bitcoin_rpc_client.sendRawTransaction(txb.build().toHex());
    }

    async send_eth_transaction_to(to, data = "0x0", value = "0x0") {
        let nonce = await web3.eth.getTransactionCount(this.eth_address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: '0x1',
            gasLimit: '0x100000',
            to: to,
            data: data,
            value: value,
            chainId: 1
        });

        tx.sign(this.eth_private_key());
        const serializedTx = tx.serialize();
        let hex = '0x' + serializedTx.toString('hex');
        return web3.eth.sendSignedTransaction(hex);
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
            chai.request(query_url).get('').end((err, res) => {
                if (err) {
                    return rej(err);
                }
                res.should.have.status(200);
                if (res.body.matches.length !== 0) {
                    final_res(res.body);
                }
                else {
                    setTimeout(() => {
                        this.poll_until_matches(chai, query_url).then((result) => {
                            final_res(result);
                        });
                    }, 200);
                }
            });
        });
    }
}


module.exports.player_conf = (name, utxo) => {
    return new PlayerConf(name, utxo);
};
module.exports.ledger_query_service_conf = (host, port) => {
    return new LedgerQueryServiceConf(host, port);
};

{
    const eth_funded_private_key = Buffer.from(process.env.ETH_FUNDED_PRIVATE_KEY, "hex");
    let nonce = 0;

    module.exports.give_eth_to = (address, value) => {
        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: '0x0',
            gasLimit: '0x5208',
            to: address,
            value: web3.utils.numberToHex(web3.utils.toWei(value.toString(), 'ether')),
            chainId: 1
        });
        tx.sign(eth_funded_private_key);
        const serializedTx = tx.serialize();
        nonce++;
        let hex = '0x' + serializedTx.toString('hex');
        return web3.eth.sendSignedTransaction('0x' + serializedTx.toString('hex'));
    }
}
