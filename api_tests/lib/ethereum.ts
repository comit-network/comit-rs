import { ECPair } from "bitcoinjs-lib";
import EthereumTx = require("ethereumjs-tx");
import BN = require("bn.js");

const Web3 = require("web3");
const bitcoin = require("bitcoinjs-lib");
const ethutil = require("ethereumjs-util");
const fs = require("fs");
const util = require("./util.js");

let _web3Client: any;
let _ethConfig: IEthConfig;

export interface IEthConfig {
    rpc_url: string;
}

function createWeb3Client(ethConfig: IEthConfig) {
    if (!ethConfig && _web3Client) {
        throw new Error("ethereum configuration is needed");
    }
    if (!_web3Client || _ethConfig !== ethConfig) {
        const httpProvider = new Web3.providers.HttpProvider(ethConfig.rpc_url);
        _web3Client = new Web3(httpProvider);
        _ethConfig = ethConfig;
    }

    return _web3Client;
}

module.exports.createClient = (ethConfig: IEthConfig) => {
    return createWeb3Client(ethConfig);
};

async function ethBalance(address: string) {
    return _web3Client.eth.getBalance(address).then((balance: string) => {
        return BigInt(balance);
    });
}

module.exports.ethBalance = async function(address: string) {
    return ethBalance(address);
};

{
    const function_identifier = "40c10f19";
    module.exports.mintErc20Tokens = (
        ownerWallet: EthereumWallet,
        contract_address: string,
        to_address: string,
        amount: BN
    ) => {
        to_address = to_address.replace(/^0x/, "").padStart(64, "0");
        const hexAmount = _web3Client.utils
            .numberToHex(amount)
            .replace(/^0x/, "")
            .padStart(64, "0");
        const payload = "0x" + function_identifier + to_address + hexAmount;

        return ownerWallet.send_eth_transaction_to(
            contract_address,
            payload,
            "0x0"
        );
    };
}

export class EthereumWallet {
    keypair: ECPair;
    _address: string;

    constructor(ethConfig: IEthConfig) {
        this.keypair = bitcoin.ECPair.makeRandom({ rng: util.test_rng });
        this._address =
            "0x" +
            ethutil.privateToAddress(this.keypair.privateKey).toString("hex");
        createWeb3Client(ethConfig);
    }

    address() {
        return this._address;
    }

    async fund(ethAmount: BN) {
        const parity_dev_account = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
        const parity_dev_password = "";
        const weiAmount = _web3Client.utils.toWei(ethAmount, "ether");
        const tx = {
            from: parity_dev_account,
            to: this.address(),
            value: _web3Client.utils.numberToHex(weiAmount),
        };
        return _web3Client.eth.personal.sendTransaction(
            tx,
            parity_dev_password
        );
    }

    async send_eth_transaction_to(
        to: string,
        data: string = "0x0",
        value: string = "0",
        gas_limit: string = "0x100000"
    ) {
        if (!to) {
            throw new Error("`to` cannot be null");
        }

        let nonce = await _web3Client.eth.getTransactionCount(this.address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit: gas_limit,
            to: to,
            data: data,
            value: _web3Client.utils.numberToHex(_web3Client.utils.toBN(value)),
            chainId: 1,
        });

        return this.signAndSend(tx);
    }

    async deploy_erc20_token_contract(projectRoot: string) {
        const token_contract_deploy =
            "0x" +
            fs
                .readFileSync(
                    projectRoot +
                        "/application/comit_node/tests/parity_client/erc20_token_contract.asm.hex",
                    "utf8"
                )
                .trim();
        return this.deploy_contract(token_contract_deploy);
    }

    async deploy_contract(data = "0x0", value = "0", gas_limit = "0x3D0900") {
        let nonce = await _web3Client.eth.getTransactionCount(this.address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit: gas_limit,
            to: null,
            data: data,
            value: _web3Client.utils.numberToHex(_web3Client.utils.toBN(value)),
            chainId: 1,
        });

        return this.signAndSend(tx);
    }

    async signAndSend(tx: EthereumTx) {
        tx.sign(this.keypair.privateKey);
        const serializedTx = tx.serialize();
        let hex = "0x" + serializedTx.toString("hex");

        return _web3Client.eth.sendSignedTransaction(hex);
    }
}

module.exports.erc20_balance = async function(
    tokenHolderAddress: string,
    contractAddress: string
) {
    const function_identifier = "70a08231";

    const padded_address = tokenHolderAddress
        .replace(/^0x/, "")
        .padStart(64, "0");
    const payload = "0x" + function_identifier + padded_address;

    const tx = {
        from: tokenHolderAddress,
        to: contractAddress,
        data: payload,
    };

    let hex_balance = await _web3Client.eth.call(tx);
    return _web3Client.utils.toBN(hex_balance);
};

export function createEthereumWallet(ethConfig: IEthConfig) {
    return new EthereumWallet(ethConfig);
}
