const bitcoin = require("bitcoinjs-lib");
const bitcoin_rpc_client_conf = require("./bitcoin_rpc_client_conf.js");
const ethutil = require("ethereumjs-util");
const EthereumTx = require("ethereumjs-tx");
const fs = require("fs");
const web3_conf = require("./web3_conf.js");

const web3 = web3_conf.create();

const logger = global.harness.logger;

const bitcoin_rpc_client = bitcoin_rpc_client_conf.create_client();

const token_contract_deploy =
    "0x" +
    fs
        .readFileSync(
            global.harness.project_root +
                "/application/comit_node/tests/parity_client/erc20_token_contract.asm.hex",
            "utf8"
        )
        .trim();

let test_rng_counter = 0;

function test_rng() {
    test_rng_counter++;
    return Buffer.from(("" + test_rng_counter).padStart(32, "0"));
}

//FIXME: Remove this whenever this change:
// https://github.com/bitcoinjs/bitcoinjs-lib/commit/44a98c0fa6487eaf81500427366787a953ff890d#diff-9e60abeb4e2333a5d2f02de53b4edfac
// Hits npm!
const regtest = {
    messagePrefix: "\x18Bitcoin Signed Message:\n",
    bech32: "bcrt",
    bip32: {
        public: 0x043587cf,
        private: 0x04358394,
    },
    pubKeyHash: 0x6f,
    scriptHash: 0xc4,
    wif: 0xef,
};

class WalletConf {
    constructor(owner) {
        this.eth_keypair = bitcoin.ECPair.makeRandom({ rng: test_rng });
        this.btc_keypair = bitcoin.ECPair.makeRandom({ rng: test_rng });
        this.bitcoin_utxos = [];
        this.owner = owner;
        this._eth_address =
            "0x" +
            ethutil
                .privateToAddress(this.eth_keypair.privateKey)
                .toString("hex");
        this._btc_identity = bitcoin.payments.p2wpkh({
            pubkey: this.btc_keypair.publicKey,
            network: regtest,
        });

        logger.trace(
            "Generated eth address for %s is %s",
            this.owner,
            this._eth_address
        );
        logger.trace(
            "Generated btc address for %s is %s",
            this.owner,
            this._btc_identity.address
        );
    }

    eth_address() {
        return this._eth_address;
    }

    btc_identity() {
        return this._btc_identity;
    }

    async fund_btc(btc_value) {
        let txid = await bitcoin_rpc_client.sendToAddress(
            this.btc_identity().address,
            btc_value
        );
        let raw_transaction = await bitcoin_rpc_client.getRawTransaction(txid);
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
        const web3 = web3_conf.create();
        const tx = {
            from: parity_dev_account,
            to: this.eth_address(),
            value: web3.utils.numberToHex(
                web3.utils.toWei(eth_amount.toString(), "ether")
            ),
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
            chainId: 1,
        });

        logger.trace(
            "Transaction %s transfers %s wei to %s",
            tx.hash().toString("hex"),
            tx.value,
            tx.to.toString("hex")
        );

        return this.sign_and_send(tx);
    }

    async deploy_erc20_token_contract() {
        return this.deploy_eth_contract(token_contract_deploy);
    }

    async deploy_eth_contract(
        data = "0x0",
        value = "0",
        gas_limit = "0x3D0900"
    ) {
        let nonce = await web3.eth.getTransactionCount(this.eth_address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit: gas_limit,
            to: null,
            data: data,
            value: web3.utils.numberToHex(value),
            chainId: 1,
        });

        let receipt = await this.sign_and_send(tx);

        let contract_balance = await web3.eth.getBalance(
            receipt.contractAddress
        );

        logger.trace(
            "Contract deployed at %s holds %s wei",
            receipt.contractAddress,
            contract_balance
        );

        return receipt;
    }

    async sign_and_send(tx) {
        tx.sign(this.eth_keypair.privateKey);
        const serializedTx = tx.serialize();
        let hex = "0x" + serializedTx.toString("hex");
        let receipt = await web3.eth.sendSignedTransaction(hex);

        logger.trace(
            "Receipt for transaction %s",
            receipt.transactionHash,
            receipt
        );

        return receipt;
    }
}

module.exports.create = (eth_private_key, utxo) => {
    return new WalletConf(eth_private_key, utxo);
};
