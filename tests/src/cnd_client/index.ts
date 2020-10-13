import axios, { AxiosInstance, AxiosPromise, AxiosResponse } from "axios";
import actionToHttpRequest, {
    FieldValueResolverFn,
} from "../action_to_http_request";
import { problemResponseInterceptor } from "../axios_rfc7807_middleware";
import { Action } from "./siren";
import {
    CreateBtcDaiOrderPayload,
    GetInfoResponse,
    HalbitHerc20Payload,
    HbitHerc20Payload,
    Herc20HalbitPayload,
    Herc20HbitPayload,
} from "./payload";

export default class CndClient {
    public readonly client: AxiosInstance;

    public constructor(cndUrl: string) {
        this.client = axios.create({
            baseURL: cndUrl,
            headers: {
                "Content-Type": "application/json",
            },
        });
        this.client.interceptors.response.use(
            (response) => response,
            problemResponseInterceptor
        );
    }

    /**
     * Get the peer id of the cnd node
     *
     * @returns Promise that resolves with the peer id of the cnd node,
     * @throws A {@link Problem} from the cnd REST API or an {@link Error}.
     */
    public async getPeerId(): Promise<string> {
        const info = await this.getInfo();
        if (!info.id) {
            throw new Error("id field not present");
        }

        return info.id;
    }

    public async dial(address: string) {
        await this.client.post("dial", { addresses: [address] });
    }

    /**
     * Get the addresses on which cnd is listening for peer-to-peer/COMIT messages.
     *
     * @returns An array of multiaddresses
     * @throws A {@link Problem} from the cnd REST API or an {@link Error}.
     */
    public async getPeerListenAddresses(): Promise<string[]> {
        const info = await this.getInfo();
        if (!info.listen_addresses) {
            throw new Error("listen addresses field not present");
        }

        return info.listen_addresses;
    }

    /**
     * Fetch data from the REST API.
     *
     * @param path The URL to GET.
     * @returns The data returned by cnd.
     * @throws A {@link Problem} from the cnd REST API or an {@link Error}.
     */
    public fetch<T>(path: string): AxiosPromise<T> {
        return this.client.get(path);
    }

    /**
     * Proceed with an action request on the cnd REST API.
     *
     * @param action The action to perform.
     * @param resolver A function that returns data needed to perform the action, this data is likely to be provided by a
     * blockchain wallet or interface (e.g. wallet address).
     * @throws A {@link Problem} from the cnd REST API, or {@link WalletError} if the blockchain wallet throws, or an {@link Error}.
     */
    public async executeSirenAction(
        action: Action,
        resolver?: FieldValueResolverFn
    ): Promise<AxiosResponse> {
        const request = await actionToHttpRequest(action, resolver);

        return this.client.request(request);
    }

    public async createHerc20Hbit(body: Herc20HbitPayload): Promise<string> {
        const response = await this.client.post(
            "swaps/herc20/hbit",
            JSON.stringify(body, bigIntSerializer)
        );

        return response.headers.location;
    }

    public async createHbitHerc20(body: HbitHerc20Payload): Promise<string> {
        const response = await this.client.post(
            "swaps/hbit/herc20",
            JSON.stringify(body, bigIntSerializer)
        );

        return response.headers.location;
    }

    public async createHalbitHerc20(
        body: HalbitHerc20Payload
    ): Promise<string> {
        const response = await this.client.post(
            "swaps/halbit/herc20",
            JSON.stringify(body, bigIntSerializer)
        );

        return response.headers.location;
    }

    public async createHerc20Halbit(
        body: Herc20HalbitPayload
    ): Promise<string> {
        const response = await this.client.post(
            "swaps/herc20/halbit",
            JSON.stringify(body, bigIntSerializer)
        );

        return response.headers.location;
    }

    public async createBtcDaiOrder(order: CreateBtcDaiOrderPayload) {
        const response = await this.client.post(
            "/orders/BTC-DAI",
            JSON.stringify(order, bigIntSerializer)
        );

        return response.headers.location;
    }

    private async getInfo(): Promise<GetInfoResponse> {
        const response = await this.client.get("/");

        return response.data;
    }
}

function bigIntSerializer(_key: string, value: any): any {
    if (typeof value === "bigint") {
        return value.toString(10);
    }

    return value;
}
