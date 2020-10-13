import { Swap } from "./swap";
import { SwapAction } from "./cnd_client/payload";

/**
 * An executable action.
 */
export class Action {
    constructor(private action: SwapAction, private swap: Swap) {}

    get name(): string {
        return this.action.name;
    }

    /**
     * Execute the action.
     *
     * @throws A {@link Problem} from the cnd REST API, or {@link WalletError} if the blockchain wallet throws, or an {@link Error}.
     */
    public async execute(): Promise<string> {
        const response = await this.swap.executeAction(this.action);
        return this.swap.doLedgerAction(response.data);
    }
}
