import { EthereumWallet } from "../wallets/ethereum";
import { BitcoinWallet } from "../wallets/bitcoin";
import { LightningChannel } from "../wallets/lightning";

export interface BalanceAsserter {
    assertReceived(): Promise<void>;
    assertSpent(): Promise<void>;
    assertRefunded(): Promise<void>;
    assertNothingReceived(): Promise<void>;
}

export class Erc20BalanceAsserter implements BalanceAsserter {
    public static async newInstance(
        wallet: EthereumWallet,
        swapAmount: bigint,
        tokenContract: string
    ) {
        await wallet.mintErc20(swapAmount, tokenContract);

        const startingBalance = await wallet.getErc20Balance(tokenContract);

        return new Erc20BalanceAsserter(
            wallet,
            startingBalance,
            swapAmount,
            tokenContract
        );
    }

    constructor(
        private readonly wallet: EthereumWallet,
        private readonly startingBalance: bigint,
        private readonly swapAmount: bigint,
        private readonly tokenContract: string
    ) {}

    public async assertReceived() {
        const currentBalance = await this.wallet.getErc20Balance(
            this.tokenContract
        );
        const expectedBalance = this.startingBalance + this.swapAmount;

        if (currentBalance !== expectedBalance) {
            throw new Error(
                `Expected ${expectedBalance} tokens in contract ${this.tokenContract} but got ${currentBalance}`
            );
        }
    }

    public async assertSpent() {
        const currentBalance = await this.wallet.getErc20Balance(
            this.tokenContract
        );
        const expectedBalance = this.startingBalance - this.swapAmount;

        if (currentBalance !== expectedBalance) {
            throw new Error(
                `Expected ${expectedBalance} tokens in contract ${this.tokenContract} but got ${currentBalance}`
            );
        }
    }

    public async assertRefunded() {
        const currentBalance = await this.wallet.getErc20Balance(
            this.tokenContract
        );

        expect(currentBalance.toString(10)).toEqual(
            this.startingBalance.toString(10)
        );
    }

    public async assertNothingReceived() {
        const currentBalance = await this.wallet.getErc20Balance(
            this.tokenContract
        );

        expect(currentBalance.toString(10)).toEqual(
            this.startingBalance.toString(10)
        );
    }
}

export class OnChainBitcoinBalanceAsserter implements BalanceAsserter {
    public static async newInstance(wallet: BitcoinWallet, swapAmount: bigint) {
        // need to mint more than what we want to swap to pay for miner fees
        await wallet.mint(swapAmount * 2n);

        const startingBalance = await wallet.getBalance();

        return new OnChainBitcoinBalanceAsserter(
            wallet,
            startingBalance,
            swapAmount
        );
    }

    constructor(
        private readonly wallet: BitcoinWallet,
        private readonly startingBalance: bigint,
        private readonly swapAmount: bigint
    ) {}

    public async assertReceived() {
        const currentBalance = await this.wallet.getBalance();
        const expectedBalance =
            this.startingBalance + this.swapAmount - this.wallet.MaximumFee;

        if (currentBalance < expectedBalance) {
            throw new Error(
                `Expected at least ${expectedBalance} sats but got ${currentBalance}`
            );
        }
    }

    public async assertSpent() {
        const currentBalance = await this.wallet.getBalance();
        const expectedBalance =
            this.startingBalance - this.swapAmount - this.wallet.MaximumFee;

        if (currentBalance < expectedBalance) {
            throw new Error(
                `Expected at least ${expectedBalance} sats but got ${currentBalance}`
            );
        }
    }

    public async assertRefunded() {
        const currentBalance = await this.wallet.getBalance();
        const expectedBalance = this.startingBalance - this.wallet.MaximumFee; // Even if we refund, we have to spend some money on fees

        expect(currentBalance).toBeGreaterThanOrEqual(expectedBalance);
    }

    public async assertNothingReceived() {
        const currentBalance = await this.wallet.getBalance();

        expect(currentBalance.toString(10)).toEqual(
            this.startingBalance.toString(10)
        );
    }
}

export class LNBitcoinBalanceAsserter implements BalanceAsserter {
    public static async newInstance(
        wallet: LightningChannel,
        swapAmount: bigint
    ) {
        const startingBalance = await wallet.getBalance();

        return new LNBitcoinBalanceAsserter(
            wallet,
            startingBalance,
            swapAmount
        );
    }

    constructor(
        private readonly wallet: LightningChannel,
        private readonly startingBalance: bigint,
        private readonly swapAmount: bigint
    ) {}

    public async assertReceived() {
        const currentBalance = await this.wallet.getBalance();
        const expectedBalance = this.startingBalance + this.swapAmount;

        if (currentBalance !== expectedBalance) {
            throw new Error(
                `Expected ${expectedBalance} sats but got ${currentBalance}`
            );
        }
    }

    public async assertSpent(): Promise<void> {
        const currentBalance = await this.wallet.getBalance();
        const expectedBalance = this.startingBalance - this.swapAmount;

        if (currentBalance !== expectedBalance) {
            throw new Error(
                `Expected ${expectedBalance} sats but got ${currentBalance}`
            );
        }
    }

    public async assertRefunded() {
        const currentBalance = await this.wallet.getBalance();

        expect(currentBalance.toString(10)).toEqual(
            this.startingBalance.toString(10)
        );
    }

    public async assertNothingReceived() {
        const currentBalance = await this.wallet.getBalance();

        expect(currentBalance.toString(10)).toEqual(
            this.startingBalance.toString(10)
        );
    }
}
