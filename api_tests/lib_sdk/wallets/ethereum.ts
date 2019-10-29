import { EthereumWallet as EthereumWalletSdk } from "comit-sdk";
import { Asset } from "comit-sdk";
import { ethers } from "ethers";
import { BigNumber, bigNumberify } from "ethers/utils";
import { EthereumNodeConfig } from "../../lib/ethereum";
import { pollUntilMinted, Wallet } from "./index";

export class EthereumWallet implements Wallet {
    public readonly inner: EthereumWalletSdk;
    public MaximumFee = 0;

    private readonly parity: ethers.Wallet;

    constructor(config: EthereumNodeConfig) {
        const provider = new ethers.providers.JsonRpcProvider(config.rpc_url);
        this.parity = new ethers.Wallet(
            "0x4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7",
            provider
        );

        this.inner = new EthereumWalletSdk(config.rpc_url);
    }

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "ether") {
            throw new Error(
                `Cannot mint asset ${asset.name} with EthereumWallet`
            );
        }

        const startingBalance = await this.getBalance();

        const minimumExpectedBalance = bigNumberify(asset.quantity);
        this.parity.sendTransaction({
            to: this.account(),
            value: minimumExpectedBalance.mul(2), // make sure we have at least twice as much
            gasLimit: 21000,
        });

        await pollUntilMinted(
            this,
            new BigNumber(startingBalance)
                .add(minimumExpectedBalance)
                .toString()
        );
    }

    public account(): string {
        return this.inner.getAccount();
    }

    public async getBalance(): Promise<number> {
        return parseInt((await this.inner.getBalance()).toString(), 10);
    }
}
