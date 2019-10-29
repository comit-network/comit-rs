import { Asset as AssetSdk } from "comit-sdk";

export interface Asset extends AssetSdk {
    name: AssetKind;
    [k: string]: any;
}

export enum AssetKind {
    Bitcoin = "bitcoin",
    Ether = "ether",
    Erc20 = "erc20",
}
