declare module "satoshi-bitcoin" {
    export function toSatoshi(btc: number | string): number;
    export function toBitcoin(sat: number | string): number;
}
