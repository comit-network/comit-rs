import {
    Action,
    EmbeddedRepresentationSubEntity,
    Entity,
    Link,
} from "comit-sdk/dist/src/cnd/siren";
import { Peer } from "comit-sdk";

/**
 * Payloads for the `/swaps/` REST API.
 */

/*
 * The payload for POST requests to create a swap on the cnd REST API.
 */
interface Payload<A, B> {
    alpha: A;
    beta: B;
    role: "Alice" | "Bob";
    peer: Peer;
}

export type HalbitHerc20Payload = Payload<HalbitPayload, Herc20Payload>;
export type Herc20HalbitPayload = Payload<Herc20Payload, HalbitPayload>;
export type HbitHerc20Payload = Payload<HbitPayload, Herc20Payload>;
export type Herc20HbitPayload = Payload<Herc20Payload, HbitPayload>;

export type HalbitPayload = {
    amount: string;
    identity: string;
    network: string;
    cltv_expiry: number;
};

export type Herc20Payload = {
    amount: string;
    identity: string;
    token_contract: string;
    absolute_expiry: number;
    chain_id: number;
};

export type HbitPayload = {
    amount: string;
    final_identity: string;
    network: string;
    absolute_expiry: number;
};

/**
 * The payload returned when fetching one swap on the `/swaps/:id` endpoint
 */

export interface SwapResponse extends Entity {
    properties: Properties;
    entities: (LedgerState | LedgerParameters)[];
    actions: SwapAction[];
    /**
     * links for this swap, contains a self reference
     */
    links: Link[];
}

/**
 * Element of the array in the payload returned when fetching all swaps on the `/swaps/` endpoint
 */
export interface SwapElementResponse extends EmbeddedRepresentationSubEntity {
    properties: Properties;
    entities: (LedgerState | LedgerParameters)[];
    actions: SwapAction[];
    /**
     * links for this swap, contains a self reference
     */
    links: Link[];
}

/**
 * The properties of a swap
 */
export interface Properties {
    /**
     * The status this swap is currently in.
     */
    status: SwapStatus;
    /**
     * The role in which you are participating in this swap.
     */
    role: "Alice" | "Bob";
}

/**
 * The overall status of a swap
 */
export enum SwapStatus {
    Created = "CREATED",
    InProgress = "IN_PROGRESS",
    Swapped = "SWAPPED",
    NotSwapped = "NOT_SWAPPED",
}

/**
 * The parameters of a given ledger
 */
export interface LedgerParameters extends EmbeddedRepresentationSubEntity {
    /**
     * The relation of these ledger parameters to the parent object (*SwapProperties).
     */
    rel: ["alpha" | "beta"];
    /**
     * Human readable title.
     */
    title: "Parameters of the Alpha Ledger" | "Parameters of the Beta Ledger";
    /**
     * Class of this sub-entity to facilitate parsing.
     */
    class: ["parameters"];
    properties: Hbit | Herc20 | Halbit;
}

export interface Hbit {
    protocol: "hbit";
    quantity: string; // In Satoshi.
}

export interface Herc20 {
    protocol: "herc20";
    quantity: string; // In Wei.
    contract_address: string;
}

export interface Halbit {
    protocol: "halbit";
    quantity: string; // In Satoshi.
}
//
/**
 * The detailed description of the ledger state.
 */
export interface LedgerState extends EmbeddedRepresentationSubEntity {
    /**
     * The relation of this ledger state to the parent object (*SwapProperties).
     */
    rel: ["alpha" | "beta"];
    /**
     * Human readable title.
     */
    title: "State of the Alpha Ledger" | "State of the Beta Ledger";
    /**
     * Class of this sub-entity to facilitate parsing.
     */
    class: ["state"];
    properties: {
        events: LedgerEvent;
        status: EscrowStatus;
    };
}

/**
 * The ledger events related to a given step.
 */
export type LedgerEvent = {
    [k in Step]: string;
};

/**
 * The status of the escrow (htlc, lightning invoice, etc) on the ledger.
 */
export enum EscrowStatus {
    /**
     * The escrow does not exist yet.
     */
    None = "NONE",
    /**
     * The escrow has been initialized.
     *
     * Initialization is a step that does not endure any cost to the user.
     */
    Initialized = "INITIALIZED",
    /**
     * The escrow has been deployed.
     *
     * Deployment is a step that endures some, relatively small, cost to the user due to computation needed on the blockchain.
     */
    Deployed = "DEPLOYED",
    /**
     * The escrow has been funded.
     *
     * Funding is a step where all the assets to be sold are sent and locked in the escrow.
     */
    Funded = "FUNDED",
    /**
     * The assets have been redeemed from the escrow.
     *
     * Redemption is a step where all the assets to be acquired are received from the escrow.
     */
    Redeemed = "REDEEMED",
    /**
     * The assets have been refunded from the escrow.
     *
     * Refunding is a step where all the assets to be sold are received back from the escrow, meaning the swap has been aborted.
     */
    Refunded = "REFUNDED",
    /**
     * An incorrect amount of assets have been sent to the escrow.
     *
     * To protect the user, if an incorrect amount of asset have been sent to the escrow, cnd will not propose redemption
     * as an option and only the refund actions will be available down the line.
     */
    IncorrectlyFunded = "INCORRECTLY_FUNDED",
}

/**
 * The possible steps needed on each side of the swap for its execution.
 *
 * Not all steps are needed for all protocols and ledgers.
 * E.g. for Han Bitcoin the steps are: fund, redeem (or refund)
 */
export enum Step {
    Init = "init",
    Deploy = "deploy",
    Fund = "fund",
    Redeem = "redeem",
    Refund = "refund",
}

/**
 * An action that is available for the given swap.
 */
export interface SwapAction extends Action {
    name: Step;
}

export enum Position {
    Buy = "buy",
    Sell = "sell",
}
