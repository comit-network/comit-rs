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

let web3Client: any;

export interface EthereumNodeConfig {
    rpc_url: string;
}

function createWeb3Client(ethConfig?: EthereumNodeConfig) {
    if (!ethConfig && web3Client) {
        throw new Error("ethereum configuration is needed");
    }
    if (!web3Client || ethConfig !== ethConfig) {
        const httpProvider = new HttpProvider(ethConfig.rpc_url);
        web3Client = new Web3(httpProvider);

        // https://github.com/ethereum/web3.js/issues/2822
        web3Client.eth.transactionConfirmationBlocks = 1;
    }

    return web3Client;
}

async function ethBalance(address: string) {
    const balance: string = await web3Client.eth.getBalance(address);
    return utils.toBN(balance);
}

async function erc20Balance(
    tokenHolderAddress: string,
    contractAddress: string
) {
    const functionIdentifier = "70a08231";

    const paddedAddress = tokenHolderAddress
        .replace(/^0x/, "")
        .padStart(64, "0");
    const payload = "0x" + functionIdentifier + paddedAddress;

    const tx = {
        from: tokenHolderAddress,
        to: contractAddress,
        data: payload,
    };

    const hexBalance = await web3Client.eth.call(tx);
    return utils.toBN(hexBalance);
}

async function mintErc20Tokens(
    ownerWallet: EthereumWallet,
    contractAddress: string,
    toAddress: string,
    amount: BN | string | number
) {
    const functionIdentifier = "40c10f19";

    toAddress = toAddress.replace(/^0x/, "").padStart(64, "0");

    if (typeof amount === "string" || typeof amount === "number") {
        amount = utils.toBN(amount);
    }

    const hexAmount = utils
        .numberToHex(amount)
        .replace(/^0x/, "")
        .padStart(64, "0");
    const payload = "0x" + functionIdentifier + toAddress + hexAmount;

    return ownerWallet.sendEthTransactionTo(contractAddress, payload, "0x0");
}

export class EthereumWallet {
    private readonly keypair: ECPair;
    private readonly account: string;

    constructor(ethConfig: EthereumNodeConfig) {
        this.keypair = bitcoin.ECPair.makeRandom({ rng: util.test_rng });
        this.account =
            "0x" +
            ethutil.privateToAddress(this.keypair.privateKey).toString("hex");
        createWeb3Client(ethConfig);
    }

    public address() {
        return this.account;
    }

    public ethBalance() {
        return ethBalance(this.account);
    }

    public erc20Balance(contractAddress: string) {
        return erc20Balance(this.account, contractAddress);
    }

    public async fund(ether: string) {
        const parityDevAccount = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
        const parityDevPassword = "";

        const weiAmount = utils.toWei(ether, "ether");
        const weiAmountBN = utils.toBN(weiAmount);

        const tx = {
            from: parityDevAccount,
            to: this.address(),
            value: utils.numberToHex(weiAmountBN),
        };
        return web3Client.eth.personal.sendTransaction(tx, parityDevPassword);
    }

    public async mintErc20To(
        toAddress: string,
        amount: BN | string | number,
        contractAddress: string
    ) {
        const receipt = await mintErc20Tokens(
            this,
            contractAddress,
            toAddress,
            amount
        );

        if (!receipt.status) {
            throw new Error(
                `Minting ${amount} tokens to address ${toAddress} failed`
            );
        }

        return receipt;
    }

    public async sendEthTransactionTo(
        to: string,
        data: string,
        value: BN | string | number = utils.toBN(0),
        gasLimit: string = "0x100000"
    ) {
        if (!to) {
            throw new Error("`to` cannot be null");
        }

        if (typeof value === "string" || typeof value === "number") {
            value = utils.toBN(value);
        }

        const nonce = await web3Client.eth.getTransactionCount(this.address());

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit,
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
        const tokenContractDeploy =
            "0x" +
            fs
                .readFileSync(
                    projectRoot +
                        "/internal/blockchain_contracts/tests/parity_client/erc20_token_contract.asm.hex",
                    "utf8"
                )
                .trim();
        const receipt = await this.deploy_contract(tokenContractDeploy);
        return receipt.contractAddress;
    }

    public async deploy_contract(
        data: string = "0x0",
        value: BN | number | string = utils.toBN(0),
        gasLimit = "0x3D0900"
    ) {
        const nonce = await web3Client.eth.getTransactionCount(this.address());

        if (typeof value === "number" || typeof value === "string") {
            value = utils.toBN(value);
        }

        const tx = new EthereumTx({
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit,
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

        return web3Client.eth.sendSignedTransaction(hex);
    }
}
