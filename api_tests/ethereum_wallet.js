const bitcoin = require("bitcoinjs-lib");
const ethutil = require("ethereumjs-util");
const EthereumTx = require("ethereumjs-tx");
const fs = require("fs");
const test_lib = require("./test_lib.js");
const web3_conf = require("./web3_conf.js");

const web3 = web3_conf.create();
const logger = test_lib.logger();

const token_contract_deploy =
    "0x" +
    fs
        .readFileSync(
            test_lib.project_root +
                "/application/comit_node/tests/parity_client/erc20_token_contract.asm.hex",
            "utf8"
        )
        .trim();

class EthereumWallet {
    constructor() {
        this.keypair = bitcoin.ECPair.makeRandom({ rng: test_lib.test_rng });
        this._address =
            "0x" +
            ethutil.privateToAddress(this.keypair.privateKey).toString("hex");
    }

    address() {
        return this._address;
    }

    async fund(eth_amount) {
        const parity_dev_account = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
        const parity_dev_password = "";
        const tx = {
            from: parity_dev_account,
            to: this.address(),
            value: web3.utils.numberToHex(
                web3.utils.toWei(eth_amount.toString(), "ether")
            ),
        };
        return web3.eth.personal.sendTransaction(tx, parity_dev_password);
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

        let nonce = await web3.eth.getTransactionCount(this.address());

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
        return this.deploy_contract(token_contract_deploy);
    }

    async deploy_contract(data = "0x0", value = "0", gas_limit = "0x3D0900") {
        let nonce = await web3.eth.getTransactionCount(this.address());

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
        tx.sign(this.keypair.privateKey);
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

module.exports.create = () => {
    return new EthereumWallet();
};
