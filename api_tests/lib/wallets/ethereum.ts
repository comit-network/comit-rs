import { BigNumber, EthereumWallet as EthereumWalletSdk } from "comit-sdk";
import { Asset } from "comit-sdk";
import { ethers } from "ethers";
import { BigNumber as BigNumberEthers } from "ethers/utils";
import { pollUntilMinted, Wallet } from "./index";
import { TransactionRequest } from "ethers/providers";
import * as fs from "fs";
import { HarnessGlobal, sleep } from "../utils";
import { EthereumNodeConfig } from "../ledgers/ethereum";

declare var global: HarnessGlobal;

export class EthereumWallet implements Wallet {
    public readonly inner: EthereumWalletSdk;
    public MaximumFee = 0;

    private readonly parity: ethers.Wallet;
    private readonly jsonRpcProvider: ethers.providers.JsonRpcProvider;

    constructor(config: EthereumNodeConfig) {
        const provider = new ethers.providers.JsonRpcProvider(config.rpc_url);
        this.parity = new ethers.Wallet(
            "0x4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7",
            provider
        );

        this.jsonRpcProvider = provider;
        this.inner = new EthereumWalletSdk(config.rpc_url);
    }

    public async mint(asset: Asset): Promise<void> {
        switch (asset.name) {
            case "ether":
                return this.mintEther(asset);
            case "erc20":
                return this.mintErc20(asset);
            default:
                throw new Error(
                    `Cannot mint asset ${asset.name} with EthereumWallet`
                );
        }
    }

    private async mintErc20(asset: Asset): Promise<void> {
        let toAddress = this.inner.getAccount();

        const functionIdentifier = "40c10f19";
        toAddress = toAddress.replace(/^0x/, "").padStart(64, "0");

        const bigNumber = ethers.utils.bigNumberify(asset.quantity);
        const hexAmount = bigNumber
            .toHexString()
            .replace(/^0x/, "")
            .padStart(64, "0");
        const data = "0x" + functionIdentifier + toAddress + hexAmount;

        const tx: TransactionRequest = {
            to: asset.token_contract,
            gasLimit: "0x100000",
            value: "0x0",
            data,
        };
        const transactionResponse = await this.sendTransaction(tx);
        const transactionReceipt = await transactionResponse.wait(1);

        if (!transactionReceipt.status) {
            throw new Error(
                `Minting ${asset.quantity} tokens to address ${toAddress} failed`
            );
        }

        if (global.verbose) {
            console.log(
                `Minted ${asset.quantity} erc20 tokens (${asset.token_contract}) for ${toAddress}`
            );
        }
    }

    private async sendTransaction(tx: TransactionRequest) {
        const release = await global.parityAccountMutex.acquire();
        try {
            return await this.parity.sendTransaction(tx);
        } finally {
            release();
        }
    }

    private async mintEther(asset: Asset): Promise<void> {
        const startingBalance = await this.getBalanceByAsset(asset);
        const minimumExpectedBalance = asset.quantity;

        // make sure we have at least twice as much
        const value = new BigNumberEthers(minimumExpectedBalance).mul(2);
        await this.sendTransaction({
            to: this.account(),
            value,
            gasLimit: 21000,
        });

        await pollUntilMinted(
            this,
            startingBalance.minus(new BigNumber(minimumExpectedBalance)),
            asset
        );
    }

    public account(): string {
        return this.inner.getAccount();
    }

    public async deployErc20TokenContract(
        projectRoot: string
    ): Promise<string> {
        const data =
            "0x" +
            fs
                .readFileSync(
                    projectRoot + "/api_tests/erc20_token_contract.asm.hex",
                    "utf8"
                )
                .trim();
        const tx: TransactionRequest = {
            gasLimit: "0x3D0900",
            value: "0x0",
            data,
        };
        const transactionResponse = await this.parity.sendTransaction(tx);
        const transactionReceipt = await transactionResponse.wait(1);
        return transactionReceipt.contractAddress;
    }

    public async getBalanceByAsset(asset: Asset): Promise<BigNumber> {
        let balance = new BigNumber(0);
        switch (asset.name) {
            case "ether":
                balance = new BigNumber(
                    (await this.inner.getBalance()).toString()
                );
                break;
            case "erc20":
                balance = await this.inner.getErc20Balance(
                    asset.token_contract,
                    0
                );
                break;
            default:
                throw new Error(
                    `Cannot read balance for asset ${asset.name} with EthereumWallet`
                );
        }
        return balance;
    }

    public async getBlockchainTime(): Promise<number> {
        const block = await this.jsonRpcProvider.send("eth_getBlockByNumber", [
            "latest",
            false,
        ]);

        return block.timestamp;
    }

    public async getTransactionStatus(txid: string): Promise<number> {
        let transaction = await this.parity.provider.getTransaction(txid);

        // Note that TransactionResponse.wait throws an Error if the transaction is failed
        // Hence we are going for a more manual method.
        do {
            await sleep(100);
            transaction = await this.parity.provider.getTransaction(txid);
        } while (transaction.confirmations === 0);

        const transactionReceipt = await this.parity.provider.getTransactionReceipt(
            txid
        );
        return transactionReceipt.status;
    }
}
