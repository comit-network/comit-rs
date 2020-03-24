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

declare var global: HarnessGlobal;

function defaultHanEthereumEtherRequestParams(
    absoluteExpiry: number
): HanEthereumEtherRequestParams {
    return {
        amount: "5000000000000000000",
        chain_id: 17,
        identity: "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        absolute_expiry: absoluteExpiry,
    };
}

function defaultHalightLightningBitcoinRequestParams(
    cltvExpiry: number,
    lndPubkey: string
): HalightLightningBitcoinRequestParams {
    return {
        amount: "10000000",
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
    peer: Peer
): HanEthereumEtherHalightLightningBitcoinRequestBody {
    const {
        alphaAbsoluteExpiry,
        betaCltvExpiry,
    } = defaultHalightHanHerc20Expiries();
    return {
        alpha: defaultHanEthereumEtherRequestParams(alphaAbsoluteExpiry),
        beta: defaultHalightLightningBitcoinRequestParams(
            betaCltvExpiry,
            lndPubkey
        ),
        role: "Alice",
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
    peer: Peer
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
        beta: defaultHanEthereumEtherRequestParams(betaAbsoluteExpiry),
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
