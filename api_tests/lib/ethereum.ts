import { ECPair } from "bitcoinjs-lib";
import bitcoin from "bitcoinjs-lib";
import BN from "bn.js";
import EthereumTx = require("ethereumjs-tx");
import ethutil from "ethereumjs-util";
import * as fs from "fs";
import Web3 from "web3";
import { HttpProvider } from "web3-providers";
import * as utils from "web3-utils";
import * as util from "./util";

let _web3Client: any;
let _ethConfig: EthereumNodeConfig;

export interface EthereumNodeConfig {
    rpc_url: string;
}

function createWeb3Client(ethConfig?: EthereumNodeConfig) {
    if (!ethConfig && _web3Client) {
        throw new Error("ethereum configuration is needed");
    }
    if (!_web3Client || _ethConfig !== ethConfig) {
        const httpProvider = new HttpProvider(ethConfig.rpc_url);
        _web3Client = new Web3(httpProvider);

        // https://github.com/ethereum/web3.js/issues/2822
        _web3Client.eth.transactionConfirmationBlocks = 1;

        _ethConfig = ethConfig;
    }

    return _web3Client;
}

async function ethBalance(address: string) {
    const balance: string = await _web3Client.eth.getBalance(address);
    return utils.toBN(balance);
}

async function erc20Balance(
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

    const hex_balance = await _web3Client.eth.call(tx);
    return utils.toBN(hex_balance);
}

async function mintErc20Tokens(
    ownerWallet: EthereumWallet,
    contract_address: string,
    to_address: string,
    amount: BN | string | number
) {
    const functionIdentifier = "40c10f19";

    to_address = to_address.replace(/^0x/, "").padStart(64, "0");

    if (typeof amount === "string" || typeof amount === "number") {
        amount = utils.toBN(amount);
    }

    const hexAmount = utils
        .numberToHex(amount)
        .replace(/^0x/, "")
        .padStart(64, "0");
    const payload = "0x" + functionIdentifier + to_address + hexAmount;

    return ownerWallet.sendEthTransactionTo(contract_address, payload, "0x0");
}

export class EthereumWallet {
    public keypair: ECPair;
    public _address: string;

    constructor(ethConfig: EthereumNodeConfig) {
        this.keypair = bitcoin.ECPair.makeRandom({ rng: util.test_rng });
        this._address =
            "0x" +
            ethutil.privateToAddress(this.keypair.privateKey).toString("hex");
        createWeb3Client(ethConfig);
    }

    public address() {
        return this._address;
    }

    public ethBalance() {
        return ethBalance(this._address);
    }

    public erc20Balance(contractAddress: string) {
        return erc20Balance(this._address, contractAddress);
    }

    public async fund(ether: string) {
        const parity_dev_account = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
        const parity_dev_password = "";

        const weiAmount = utils.toWei(ether, "ether");
        const weiAmountBN = utils.toBN(weiAmount);

        const tx = {
            from: parity_dev_account,
            to: this.address(),
            value: utils.numberToHex(weiAmountBN),
        };
        return _web3Client.eth.personal.sendTransaction(
            tx,
            parity_dev_password
        );
    }

    public async mintErc20To(
        to_address: string,
        amount: BN | string | number,
        contract_address: string
    ) {
        const receipt = await mintErc20Tokens(
            this,
            contract_address,
            to_address,
            amount
        );

        if (!receipt.status) {
            throw new Error(
                `Minting ${amount} tokens to address ${to_address} failed`
            );
        }

        return receipt;
    }

    public async sendEthTransactionTo(
        to: string,
        data: string,
        value: BN | string | number = utils.toBN(0),
        gas_limit: string = "0x100000"
    ) {
        if (!to) {
            throw new Error("`to` cannot be null");
        }

        if (typeof value === "string" || typeof value === "number") {
            value = utils.toBN(value);
        }

        const nonce = await _web3Client.eth.getTransactionCount(this.address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit: gas_limit,
            to,
            data,
            value: utils.numberToHex(value),
            chainId: 1,
        });

        return this.signAndSend(tx);
    }

    public async deployErc20TokenContract(
        projectRoot: string
    ): Promise<string> {
        const token_contract_deploy =
            "0x" +
            fs
                .readFileSync(
                    projectRoot +
                        "/vendor/blockchain_contracts/tests/parity_client/erc20_token_contract.asm.hex",
                    "utf8"
                )
                .trim();
        const receipt = await this.deploy_contract(token_contract_deploy);
        return receipt.contractAddress;
    }

    public async deploy_contract(
        data: string = "0x0",
        value: BN | number | string = utils.toBN(0),
        gas_limit = "0x3D0900"
    ) {
        const nonce = await _web3Client.eth.getTransactionCount(this.address());

        if (typeof value === "number" || typeof value === "string") {
            value = utils.toBN(value);
        }

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit: gas_limit,
            to: null,
            data,
            value: utils.numberToHex(value),
            chainId: 1,
        });

        return this.signAndSend(tx);
    }

    public async signAndSend(tx: EthereumTx) {
        tx.sign(this.keypair.privateKey);
        const serializedTx = tx.serialize();
        const hex = "0x" + serializedTx.toString("hex");

        return _web3Client.eth.sendSignedTransaction(hex);
    }
}
