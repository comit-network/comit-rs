const bitcoin = require("bitcoinjs-lib");
const ethutil = require("ethereumjs-util");
const EthereumTx = require("ethereumjs-tx");
const fs = require("fs");
const Web3 = require("web3");
const util = require("./util.js");
const logger = global.harness.logger;
const eth_config = global.harness.ledgers_config.ethereum;
const web3 = new Web3(new Web3.providers.HttpProvider(eth_config.rpc_url));

module.exports.web3 = web3;

async function eth_balance(address) {
    return web3.eth.getBalance(address).then(balance => {
        return BigInt(balance);
    });
}

module.exports.eth_balance = async function(address) {
    return eth_balance(address);
};

module.exports.log_eth_balance = async function(
    when,
    player,
    address,
    address_type
) {
    logger.info(
        "%s the swap, %s has %s wei at the %s address %s",
        when,
        player,
        await eth_balance(address),
        address_type,
        address
    );
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
            .numberToHex(amount.toString())
            .replace(/^0x/, "")
            .padStart(64, "0");
        const payload = "0x" + function_identifier + to_address + amount;

        return owner_wallet
            .eth()
            .send_eth_transaction_to(contract_address, payload, "0x0");
    };
}

const token_contract_deploy =
    "0x" +
    fs
        .readFileSync(
            global.harness.project_root +
                "/application/comit_node/tests/parity_client/erc20_token_contract.asm.hex",
            "utf8"
        )
        .trim();

class EthereumWallet {
    constructor() {
        this.keypair = bitcoin.ECPair.makeRandom({ rng: util.test_rng });
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

module.exports.erc20_balance = async function(
    token_holder_address,
    contract_address
) {
    const function_identifier = "70a08231";

    const padded_address = token_holder_address
        .replace(/^0x/, "")
        .padStart(64, "0");
    const payload = "0x" + function_identifier + padded_address;

    const tx = {
        from: token_holder_address,
        to: contract_address,
        data: payload,
    };

    let hex_balance = await web3.eth.call(tx);
    return BigInt(web3.utils.toBN(hex_balance).toString());
};

module.exports.create = () => {
    return new EthereumWallet();
};
