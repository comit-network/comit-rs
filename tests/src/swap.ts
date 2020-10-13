import { AxiosResponse } from "axios";
import { Action } from "./action";
import { LedgerAction } from "./cnd_client/action_payload";
import CndClient from "./cnd_client";
import { Action as SirenAction, Field } from "./cnd_client/siren";
import { AllWallets, Wallets } from "./wallets";
import { BigNumber } from "ethers";
import { SwapEntity } from "./cnd_client/payload";

export class WalletError extends Error {
    constructor(
        public readonly attemptedAction: string,
        public readonly source: Error,
        public readonly callParams: any
    ) {
        super(source.message);
    }
}

/**
 * A stateful class that represents a single swap.
 *
 * It has all the dependencies embedded that are necessary for taking actions on the swap.
 */
export class Swap {
    private readonly wallets: Wallets;

    constructor(
        private readonly cnd: CndClient,
        readonly self: string,
        wallets: AllWallets
    ) {
        this.wallets = new Wallets(wallets);
    }

    /**
     * Retrieves the next recommended action of this {@link Swap} if there is any.
     *
     * @returns An {@link Action} that can be executed or null if no action is currently recommended.
     */
    public async nextAction(): Promise<Action | null> {
        const actions = await this.cnd
            .fetch<SwapEntity>(this.self)
            .then((response) => response.data.actions);

        if (!actions || actions.length === 0) {
            return null;
        }

        if (actions.length !== 1) {
            throw new Error(
                "Several actions returned by cnd, be sure to use cnd version 0.8.0 or above."
            );
        }

        return new Action(actions[0], this);
    }

    /**
     * Low level API for executing a ledger action returned from {@link CndClient}.
     *
     * Uses the wallets given in the constructor to send transactions according to the given ledger action.
     *
     * @param ledgerAction The ledger action returned from {@link CndClient}.
     * @throws A {@link WalletError} if a wallet or blockchain action failed.
     */
    public async doLedgerAction(ledgerAction: LedgerAction): Promise<string> {
        switch (ledgerAction.type) {
            case "bitcoin-broadcast-signed-transaction": {
                const { hex, network } = ledgerAction.payload;

                try {
                    return await this.wallets.bitcoin.broadcastTransaction(
                        hex,
                        network
                    );
                } catch (error) {
                    throw new WalletError(ledgerAction.type, error, {
                        hex,
                        network,
                    });
                }
            }
            case "bitcoin-send-amount-to-address": {
                const { to, amount, network } = ledgerAction.payload;
                const sats = parseInt(amount, 10);

                try {
                    return await this.wallets.bitcoin.sendToAddress(
                        to,
                        sats,
                        network
                    );
                } catch (error) {
                    throw new WalletError(ledgerAction.type, error, {
                        to,
                        sats,
                        network,
                    });
                }
            }
            case "ethereum-call-contract": {
                const {
                    data,
                    contract_address,
                    gas_limit,
                    chain_id,
                } = ledgerAction.payload;

                try {
                    const transactionReceipt = await this.wallets.ethereum.callContract(
                        data,
                        contract_address,
                        gas_limit,
                        chain_id
                    );
                    return transactionReceipt.transactionHash;
                } catch (error) {
                    throw new WalletError(ledgerAction.type, error, {
                        data,
                        contract_address,
                        gas_limit,
                    });
                }
            }
            case "ethereum-deploy-contract": {
                const {
                    amount,
                    data,
                    gas_limit,
                    chain_id,
                } = ledgerAction.payload;
                const value = BigNumber.from(amount);

                try {
                    const transactionId = await this.wallets.ethereum.deployContract(
                        data,
                        value,
                        gas_limit,
                        chain_id
                    );
                    return transactionId.transactionHash;
                } catch (error) {
                    throw new WalletError(ledgerAction.type, error, {
                        data,
                        value,
                        gas_limit,
                    });
                }
            }
            case "lnd-send-payment": {
                const {
                    self_public_key,
                    to_public_key,
                    amount,
                    secret_hash,
                    final_cltv_delta,
                    chain,
                    network,
                } = ledgerAction.payload;

                try {
                    await this.wallets.lightning.assertLndDetails(
                        self_public_key,
                        chain,
                        network
                    );

                    await this.wallets.lightning.sendPayment(
                        to_public_key,
                        amount,
                        secret_hash,
                        final_cltv_delta
                    );

                    return secret_hash;
                } catch (error) {
                    throw new WalletError(ledgerAction.type, error, {
                        self_public_key,
                        to_public_key,
                        amount,
                        secret_hash,
                        final_cltv_delta,
                        chain,
                        network,
                    });
                }
            }
            case "lnd-add-hold-invoice": {
                const {
                    self_public_key,
                    amount,
                    secret_hash,
                    expiry,
                    cltv_expiry,
                    chain,
                    network,
                } = ledgerAction.payload;

                try {
                    await this.wallets.lightning.assertLndDetails(
                        self_public_key,
                        chain,
                        network
                    );

                    return this.wallets.lightning.addHoldInvoice(
                        amount,
                        secret_hash,
                        expiry,
                        cltv_expiry
                    );
                } catch (error) {
                    throw new WalletError(ledgerAction.type, error, {
                        self_public_key,
                        amount,
                        secret_hash,
                        expiry,
                        cltv_expiry,
                        chain,
                        network,
                    });
                }
            }
            case "lnd-settle-invoice": {
                const {
                    self_public_key,
                    secret,
                    chain,
                    network,
                } = ledgerAction.payload;
                try {
                    await this.wallets.lightning.assertLndDetails(
                        self_public_key,
                        chain,
                        network
                    );

                    await this.wallets.lightning.settleInvoice(secret);

                    return secret;
                } catch (error) {
                    throw new WalletError(ledgerAction.type, error, {
                        self_public_key,
                        secret,
                        chain,
                        network,
                    });
                }
            }
            default:
                throw new Error(`Cannot handle ${ledgerAction.type}`);
        }
    }

    public async executeAction(action: SirenAction): Promise<AxiosResponse> {
        return this.cnd.executeSirenAction(action!, async (field: Field) => {
            try {
                // Awaiting here allows us to have better context
                return await this.fieldValueResolver(field);
            } catch (error) {
                throw new WalletError(action.name, error, field);
            }
        });
    }

    private async fieldValueResolver(
        field: Field
    ): Promise<string | undefined> {
        const classes: string[] = field.class;

        if (classes.includes("bitcoin") && classes.includes("address")) {
            return this.wallets.bitcoin.getAddress();
        }

        if (classes.includes("bitcoin") && classes.includes("feePerWU")) {
            return this.wallets.bitcoin.getFee();
        }

        if (classes.includes("ethereum") && classes.includes("address")) {
            return this.wallets.ethereum.getAccount();
        }
    }
}

/**
 * Defines the parameters (for how long and how often) to try executing an action of a {@link Swap}.
 */
export interface TryParams {
    maxTimeoutSecs: number;
    tryIntervalSecs: number;
}
