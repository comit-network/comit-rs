import { HarnessGlobal } from "../utils";
import {
    HalightLightningBitcoinHanEthereumEtherRequestBody,
    HalightLightningBitcoinHerc20EthereumErc20RequestBody,
    HanEthereumEtherHalightLightningBitcoinRequestBody,
    Herc20EthereumErc20HalightLightningBitcoinRequestBody,
    HanEthereumEtherRequestParams,
    HalightLightningBitcoinRequestParams,
    Herc20EthereumErc20RequestParams,
    Peer,
} from "comit-sdk";
import { Asset, AssetKind } from "../asset";
import { Ledger, LedgerKind } from "../ledgers/ledger";
import { parseEther } from "ethers/utils";

declare var global: HarnessGlobal;

function defaultHanEthereumEtherRequestParams(
    absoluteExpiry: number,
    identity: string
): HanEthereumEtherRequestParams {
    return {
        amount: "5000000000000000000",
        chain_id: 17,
        identity,
        absolute_expiry: absoluteExpiry,
    };
}

function defaultHalightLightningBitcoinRequestParams(
    cltvExpiry: number,
    lndPubkey: string
): HalightLightningBitcoinRequestParams {
    return {
        amount: "10000",
        network: "regtest",
        identity: lndPubkey,
        cltv_expiry: cltvExpiry,
    };
}

function defaultHerc20EthereumErc20RequestParams(
    absoluteExpiry: number
): Herc20EthereumErc20RequestParams {
    return {
        amount: "9000000000000000000",
        contract_address: "0xB97048628DB6B661D4C2aA833e95Dbe1A905B280",
        chain_id: 17,
        identity: "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        absolute_expiry: absoluteExpiry,
    };
}
export function defaultHanEthereumEtherHalightLightningBitcoin(
    lndPubkey: string,
    peer: Peer,
    role: "Alice" | "Bob",
    ethereumIdentity: string
): HanEthereumEtherHalightLightningBitcoinRequestBody {
    const {
        alphaAbsoluteExpiry,
        betaCltvExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHanEthereumEtherRequestParams(
            alphaAbsoluteExpiry,
            ethereumIdentity
        ),
        beta: defaultHalightLightningBitcoinRequestParams(
            betaCltvExpiry,
            lndPubkey
        ),
        role,
        peer,
    };
}

export function defaultHerc20EthereumErc20HalightLightningBitcoin(
    lndPubkey: string,
    peer: Peer
): Herc20EthereumErc20HalightLightningBitcoinRequestBody {
    const {
        alphaAbsoluteExpiry,
        betaCltvExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHerc20EthereumErc20RequestParams(alphaAbsoluteExpiry),
        beta: defaultHalightLightningBitcoinRequestParams(
            betaCltvExpiry,
            lndPubkey
        ),
        role: "Alice",
        peer,
    };
}

export function defaultHalightLightningBitcoinHanEthereumEther(
    lndPubkey: string,
    peer: Peer,
    ethereumIdentity: string
): HalightLightningBitcoinHanEthereumEtherRequestBody {
    const {
        alphaCltvExpiry,
        betaAbsoluteExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHalightLightningBitcoinRequestParams(
            alphaCltvExpiry,
            lndPubkey
        ),
        beta: defaultHanEthereumEtherRequestParams(
            betaAbsoluteExpiry,
            ethereumIdentity
        ),
        role: "Alice",
        peer,
    };
}

export function defaultHalightLightningBitcoinHerc20EthereumErc20(
    lndPubkey: string,
    peer: Peer
): HalightLightningBitcoinHerc20EthereumErc20RequestBody {
    const {
        alphaCltvExpiry,
        betaAbsoluteExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHalightLightningBitcoinRequestParams(
            alphaCltvExpiry,
            lndPubkey
        ),
        beta: defaultHerc20EthereumErc20RequestParams(betaAbsoluteExpiry),
        role: "Alice",
        peer,
    };
}

function defaultHalightHanHerc20Expiries() {
    const alphaAbsoluteExpiry = Math.round(Date.now() / 1000) + 8;
    const betaAbsoluteExpiry = Math.round(Date.now() / 1000) + 3;

    return {
        alphaAbsoluteExpiry,
        betaAbsoluteExpiry,
        alphaCltvExpiry: 35,
        betaCltvExpiry: 35,
    };
}

export function defaultLedgerKindForAsset(asset: AssetKind): LedgerKind {
    switch (asset) {
        case AssetKind.Bitcoin:
            return LedgerKind.Bitcoin;
        case AssetKind.Ether:
            return LedgerKind.Ethereum;
        case AssetKind.Erc20:
            return LedgerKind.Ethereum;
    }
}

/**
 * WIP as the cnd REST API routes for lightning are not yet defined.
 * @param ledger
 * @returns The ledger formatted as needed for the request body to cnd HTTP API on the lightning route.
 */
export function defaultLedgerDescriptionForLedger(ledger: LedgerKind): Ledger {
    switch (ledger) {
        case LedgerKind.Lightning: {
            return {
                name: LedgerKind.Lightning,
            };
        }
        case LedgerKind.Bitcoin: {
            return {
                name: LedgerKind.Bitcoin,
                network: "regtest",
            };
        }
        case LedgerKind.Ethereum: {
            return {
                name: LedgerKind.Ethereum,
                chain_id: 17,
            };
        }
    }
}

export function defaultAssetDescription(
    asset: AssetKind,
    ledger: LedgerKind
): Asset {
    switch (asset) {
        case AssetKind.Bitcoin: {
            return {
                name: AssetKind.Bitcoin,
                ledger,
                quantity: "10000000",
            };
        }
        case AssetKind.Ether: {
            return {
                name: AssetKind.Ether,
                ledger,
                quantity: parseEther("10").toString(),
            };
        }
        case AssetKind.Erc20: {
            return {
                name: AssetKind.Erc20,
                ledger,
                quantity: parseEther("100").toString(),
                token_contract: global.tokenContract,
            };
        }
    }
}

export function defaultExpiryTimes() {
    const alphaExpiry = Math.round(Date.now() / 1000) + 8;
    const betaExpiry = Math.round(Date.now() / 1000) + 3;

    return {
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
    };
}
