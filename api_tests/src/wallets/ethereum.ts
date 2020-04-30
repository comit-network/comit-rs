import { BigNumber, EthereumWallet as EthereumWalletSdk } from "comit-sdk";
import { Asset } from "comit-sdk";
import { ethers } from "ethers";
import { BigNumber as BigNumberEthers } from "ethers/utils";
import { Wallet } from "./index";
import { TransactionRequest } from "ethers/providers";
import { sleep } from "../utils";
import { Logger } from "log4js";
import { lock } from "proper-lockfile";

export class EthereumWallet implements Wallet {
    public MaximumFee = 0;

    private constructor(
        private readonly ethereumDevAccount: ethers.Wallet,
        private readonly logger: Logger,
        private readonly ethereumLockDir: string,
        private readonly jsonRpcProvider: ethers.providers.JsonRpcProvider,
        public readonly inner: EthereumWalletSdk,
        public readonly chainId: number
    ) {}

    public static async new_instance(
        devAccountKey: string,
        rpcUrl: string,
        logger: Logger,
        ethereumLockDir: string,
        chainId: number
    ) {
        const provider = new ethers.providers.JsonRpcProvider(rpcUrl);

        const ethereumDevAccount = await ethers.Wallet.fromEncryptedJson(
            devAccountKey,
            ""
        );
        const inner = new EthereumWalletSdk(rpcUrl);
        return new EthereumWallet(
            ethereumDevAccount,
            logger,
            ethereumLockDir,
            provider,
            inner,
            chainId
        );
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

        await this.sendTransaction(tx);

        this.logger.info(
            "Minted",
            asset.quantity,
            "erc20 tokens (",
            asset.token_contract,
            ") for",
            toAddress
        );
    }

    private async sendTransaction(tx: TransactionRequest) {
        const release = await lock(this.ethereumLockDir, {
            retries: {
                retries: 10,
                minTimeout: 50,
                maxTimeout: 2000,
            },
            lockfilePath: "ethereum-dev-account",
        });

        this.logger.debug(
            "Acquired lock for ethereum-dev-account, sending transaction ",
            tx
        );

        try {
            tx.nonce = await this.jsonRpcProvider.getTransactionCount(
                this.ethereumDevAccount.address
            );
            const signedTx = await this.ethereumDevAccount.sign(tx);
            const transactionResponse = await this.jsonRpcProvider.sendTransaction(
                signedTx
            );

            this.logger.debug(
                "Transaction: ",
                transactionResponse.hash,
                " sent, waiting to be confirmed."
            );

            const transactionReceipt = await this.jsonRpcProvider.waitForTransaction(
                transactionResponse.hash,
                1
            );
            if (!transactionReceipt.status) {
                throw new Error(
                    `Transaction ${transactionReceipt.transactionHash} failed with status 1`
                );
            }
            this.logger.debug(
                "Transaction: ",
                transactionReceipt.transactionHash,
                " confirmed in block: ",
                transactionReceipt.blockHash
            );
            return transactionReceipt;
        } finally {
            await release();
        }

        this.logger.debug("Lock for ethereum-dev-account account released.");
    }

    private async mintEther(asset: Asset): Promise<void> {
        const minimumExpectedBalance = asset.quantity;

        // make sure we have at least twice as much
        const value = new BigNumberEthers(minimumExpectedBalance).mul(2);
        await this.sendTransaction({
            to: this.account(),
            value,
            gasLimit: 21000,
            chainId: this.chainId,
        });

        const balance = await this.getBalanceByAsset(asset);

        if (balance.lte(minimumExpectedBalance)) {
            throw new Error("Failed to mint Ether");
        }

        this.logger.info("Minted", asset.quantity, "ether for", this.account());
    }

    public account(): string {
        return this.inner.getAccount();
    }

    public async deployErc20TokenContract(): Promise<string> {
        const data = ERC20_CONTRACT;

        const tx: TransactionRequest = {
            gasLimit: "0x3D0900",
            value: "0x0",
            data,
            nonce: await this.jsonRpcProvider.getTransactionCount(
                this.ethereumDevAccount.address
            ),
            chainId: this.chainId,
        };
        const signedTx = await this.ethereumDevAccount.sign(tx);
        const transactionResponse = await this.jsonRpcProvider.sendTransaction(
            signedTx
        );
        const transactionReceipt = await this.jsonRpcProvider.waitForTransaction(
            transactionResponse.hash,
            1
        );
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
        let transaction = await this.jsonRpcProvider.getTransaction(txid);

        // Note that TransactionResponse.wait throws an Error if the transaction is failed
        // Hence we are going for a more manual method.
        do {
            await sleep(100);
            transaction = await this.jsonRpcProvider.getTransaction(txid);
        } while (transaction.confirmations === 0);

        const transactionReceipt = await this.jsonRpcProvider.getTransactionReceipt(
            txid
        );
        return transactionReceipt.status;
    }
}

const ERC20_CONTRACT =
    "0x60806040526000600760006101000a81548160ff0219169083151502179055503480156200002c57600080fd5b506040805190810160405280600b81526020017f50726f666974546f6b656e0000000000000000000000000000000000000000008152506040805190810160405280600381526020017f505254000000000000000000000000000000000000000000000000000000000081525060128260039080519060200190620000b3929190620001b0565b508160049080519060200190620000cc929190620001b0565b5080600560006101000a81548160ff021916908360ff1602179055505050506200010f336006620001156401000000000262001779179091906401000000009004565b6200025f565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff16141515156200015257600080fd5b60018260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b828054600181600116156101000203166002900490600052602060002090601f016020900481019282601f10620001f357805160ff191683800117855562000224565b8280016001018555821562000224579182015b828111156200022357825182559160200191906001019062000206565b5b50905062000233919062000237565b5090565b6200025c91905b80821115620002585760008160009055506001016200023e565b5090565b90565b61196d806200026f6000396000f3006080604052600436106100f1576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff16806305d2035b146100f657806306fdde0314610125578063095ea7b3146101b557806318160ddd1461021a57806323b872dd14610245578063313ce567146102ca57806339509351146102fb57806340c10f191461036057806370a08231146103c55780637d64bcb41461041c57806395d89b411461044b578063983b2d56146104db578063986502751461051e578063a457c2d714610535578063a9059cbb1461059a578063aa271e1a146105ff578063dd62ed3e1461065a575b600080fd5b34801561010257600080fd5b5061010b6106d1565b604051808215151515815260200191505060405180910390f35b34801561013157600080fd5b5061013a6106e8565b6040518080602001828103825283818151815260200191508051906020019080838360005b8381101561017a57808201518184015260208101905061015f565b50505050905090810190601f1680156101a75780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b3480156101c157600080fd5b50610200600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291908035906020019092919050505061078a565b604051808215151515815260200191505060405180910390f35b34801561022657600080fd5b5061022f6108b7565b6040518082815260200191505060405180910390f35b34801561025157600080fd5b506102b0600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001909291905050506108c1565b604051808215151515815260200191505060405180910390f35b3480156102d657600080fd5b506102df610c7c565b604051808260ff1660ff16815260200191505060405180910390f35b34801561030757600080fd5b50610346600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610c93565b604051808215151515815260200191505060405180910390f35b34801561036c57600080fd5b506103ab600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610eca565b604051808215151515815260200191505060405180910390f35b3480156103d157600080fd5b50610406600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610f10565b6040518082815260200191505060405180910390f35b34801561042857600080fd5b50610431610f58565b604051808215151515815260200191505060405180910390f35b34801561045757600080fd5b50610460610fd8565b6040518080602001828103825283818151815260200191508051906020019080838360005b838110156104a0578082015181840152602081019050610485565b50505050905090810190601f1680156104cd5780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b3480156104e757600080fd5b5061051c600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505061107a565b005b34801561052a57600080fd5b506105336110e8565b005b34801561054157600080fd5b50610580600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001909291905050506110fe565b604051808215151515815260200191505060405180910390f35b3480156105a657600080fd5b506105e5600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050611335565b604051808215151515815260200191505060405180910390f35b34801561060b57600080fd5b50610640600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050611555565b604051808215151515815260200191505060405180910390f35b34801561066657600080fd5b506106bb600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050611572565b6040518082815260200191505060405180910390f35b6000600760009054906101000a900460ff16905090565b606060038054600181600116156101000203166002900480601f0160208091040260200160405190810160405280929190818152602001828054600181600116156101000203166002900480156107805780601f1061075557610100808354040283529160200191610780565b820191906000526020600020905b81548152906001019060200180831161076357829003601f168201915b5050505050905090565b60008073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff16141515156107c757600080fd5b81600160003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508273ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925846040518082815260200191505060405180910390a36001905092915050565b6000600254905090565b60008060008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054821115151561091057600080fd5b600160008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054821115151561099b57600080fd5b600073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff16141515156109d757600080fd5b610a28826000808773ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020546115f990919063ffffffff16565b6000808673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002081905550610abb826000808673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205461161a90919063ffffffff16565b6000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002081905550610b8c82600160008773ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020546115f990919063ffffffff16565b600160008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508273ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef846040518082815260200191505060405180910390a3600190509392505050565b6000600560009054906101000a900460ff16905090565b60008073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff1614151515610cd057600080fd5b610d5f82600160003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205461161a90919063ffffffff16565b600160003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508273ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925600160003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008773ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020546040518082815260200191505060405180910390a36001905092915050565b6000610ed533611555565b1515610ee057600080fd5b600760009054906101000a900460ff16151515610efc57600080fd5b610f06838361163b565b6001905092915050565b60008060008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020549050919050565b6000610f6333611555565b1515610f6e57600080fd5b600760009054906101000a900460ff16151515610f8a57600080fd5b6001600760006101000a81548160ff0219169083151502179055507fb828d9b5c78095deeeeff2eca2e5d4fe046ce3feb4c99702624a3fd384ad2dbc60405160405180910390a16001905090565b606060048054600181600116156101000203166002900480601f0160208091040260200160405190810160405280929190818152602001828054600181600116156101000203166002900480156110705780601f1061104557610100808354040283529160200191611070565b820191906000526020600020905b81548152906001019060200180831161105357829003601f168201915b5050505050905090565b61108333611555565b151561108e57600080fd5b6110a281600661177990919063ffffffff16565b8073ffffffffffffffffffffffffffffffffffffffff167f6ae172837ea30b801fbfcdd4108aa1d5bf8ff775444fd70256b44e6bf3dfc3f660405160405180910390a250565b6110fc33600661181390919063ffffffff16565b565b60008073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff161415151561113b57600080fd5b6111ca82600160003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020546115f990919063ffffffff16565b600160003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508273ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925600160003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008773ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020546040518082815260200191505060405180910390a36001905092915050565b60008060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054821115151561138457600080fd5b600073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff16141515156113c057600080fd5b611411826000803373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020546115f990919063ffffffff16565b6000803373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055506114a4826000808673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205461161a90919063ffffffff16565b6000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508273ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef846040518082815260200191505060405180910390a36001905092915050565b600061156b8260066118ad90919063ffffffff16565b9050919050565b6000600160008473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054905092915050565b60008083831115151561160b57600080fd5b82840390508091505092915050565b600080828401905083811015151561163157600080fd5b8091505092915050565b60008273ffffffffffffffffffffffffffffffffffffffff161415151561166157600080fd5b6116768160025461161a90919063ffffffff16565b6002819055506116cd816000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205461161a90919063ffffffff16565b6000808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508173ffffffffffffffffffffffffffffffffffffffff16600073ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040518082815260200191505060405180910390a35050565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff16141515156117b557600080fd5b60018260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff161415151561184f57600080fd5b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b60008073ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff16141515156118ea57600080fd5b8260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060009054906101000a900460ff169050929150505600a165627a7a72305820cc38c3be3baa4284d7d1551695c68e8cc7c3d0dfbd17d5fe2d47c1dbe9b52b320029";
