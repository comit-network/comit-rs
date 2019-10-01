import { ECPair, ECPairInterface } from "bitcoinjs-lib";
import { ethers } from "ethers";
import { JsonRpcProvider, TransactionRequest } from "ethers/providers";
import { BigNumber } from "ethers/utils";
import * as fs from "fs";
import * as util from "./util";

let ethersClient: JsonRpcProvider;

export interface EthereumNodeConfig {
    rpc_url: string;
    network: string;
}

function createEthereumClient(ethConfig: EthereumNodeConfig) {
    if (!ethConfig && ethersClient) {
        throw new Error("ethereum configuration is needed");
    }
    if (!ethersClient || ethConfig !== ethConfig) {
        ethersClient = new ethers.providers.JsonRpcProvider(ethConfig.rpc_url);
    }

    return ethersClient;
}

async function ethBalance(address: string) {
    return ethersClient.getBalance(address);
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

    const transactionReceipt = await ethersClient.call(tx);
    return ethers.utils.bigNumberify(transactionReceipt);
}

async function mintErc20Tokens(
    ownerWallet: EthereumWallet,
    contractAddress: string,
    toAddress: string,
    amount: BigNumber | string | number
) {
    const functionIdentifier = "40c10f19";

    toAddress = toAddress.replace(/^0x/, "").padStart(64, "0");

    const bigNumber = ethers.utils.bigNumberify(amount);
    const hexAmount = bigNumber
        .toHexString()
        .replace(/^0x/, "")
        .padStart(64, "0");
    const payload = "0x" + functionIdentifier + toAddress + hexAmount;

    const transactionResponse = await ownerWallet.sendEthTransactionTo(
        contractAddress,
        payload,
        "0x0"
    );
    return transactionResponse.wait(1);
}

export class EthereumWallet {
    private readonly keypair: ECPairInterface;
    private readonly account: string;

    constructor(ethConfig: EthereumNodeConfig) {
        this.keypair = ECPair.makeRandom({ rng: util.test_rng });
        this.account = new ethers.Wallet(this.keypair.privateKey).address;
        createEthereumClient(ethConfig);
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

    public async fund(ether: string, confirmation: number = 1) {
        const parityPrivateKey =
            "0x4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7";

        const weiAmount = ethers.utils.parseEther(ether);
        const chainId = await ethersClient.getNetwork();
        const tx: TransactionRequest = {
            to: this.address(),
            value: weiAmount.toHexString(),
            gasLimit: 21000,
            chainId: chainId.chainId,
        };

        const wallet = new ethers.Wallet(parityPrivateKey, ethersClient);
        const transactionResponse = await wallet.sendTransaction(tx);
        return transactionResponse.wait(confirmation);
    }

    public async mintErc20To(
        toAddress: string,
        amount: BigNumber | string | number,
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
        value: BigNumber | string | number = 0,
        gasLimit: string = "0x100000"
    ) {
        if (!to) {
            throw new Error("`to` cannot be null");
        }

        value = ethers.utils.bigNumberify(value);

        const chainId = await ethersClient.getNetwork();
        const tx: TransactionRequest = {
            gasPrice: "0x0",
            gasLimit,
            to,
            data,
            value: value.toHexString(),
            chainId: chainId.chainId,
        };
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
                        "/blockchain_contracts/tests/parity_client/erc20_token_contract.asm.hex",
                    "utf8"
                )
                .trim();
        const transactionResponse = await this.deploy_contract(
            tokenContractDeploy
        );
        const transactionReceipt = await transactionResponse.wait(1);
        return transactionReceipt.contractAddress;
    }

    public async deploy_contract(
        data: string = "0x0",
        value: BigNumber | number | string = "0",
        gasLimit = "0x3D0900"
    ) {
        const nonce = await ethersClient.getTransactionCount(this.address());

        value = ethers.utils.bigNumberify(value);

        const tx: TransactionRequest = {
            nonce: "0x" + nonce.toString(16),
            gasPrice: "0x0",
            gasLimit,
            data,
            value: value.toHexString(),
            chainId: 17,
        };

        return this.signAndSend(tx);
    }

    public async signAndSend(tx: TransactionRequest) {
        const wallet = new ethers.Wallet(this.keypair.privateKey, ethersClient);
        tx.nonce = await wallet.getTransactionCount("latest");
        const signedTx = await wallet.sign(tx);
        return await ethersClient.sendTransaction(signedTx);
    }
}
