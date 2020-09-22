import { Action } from "../src/action";
import { Swap } from "../src/swap";
import { SwapAction } from "../src/cnd_client/payload";

const swapAction: SwapAction = { name: "redeem", href: "/redeem" };

describe("Action", () => {
    it("has the same name than the swap action used to construct", () => {
        const swap = (undefined as unknown) as Swap;
        const action = new Action(swapAction, swap);

        expect(action.name).toEqual(swapAction.name);
    });

    it("calls Swap.executeAction with the parameters of the swap action used to construct", async () => {
        const mockExecuteAction = jest.fn(async () => {
            return {};
        });
        const mockNextAction = jest.fn(async () => {
            return swapAction;
        });
        const mockDoLedgerAction = jest.fn(async () => {
            return "";
        });
        const swap = ({
            executeAction: mockExecuteAction,
            nextAction: mockNextAction,
            doLedgerAction: mockDoLedgerAction,
        } as unknown) as Swap;
        const action = new Action(swapAction, swap);

        await action.execute();

        expect(mockExecuteAction.mock.calls.length).toBe(1);
        // @ts-ignore: mockExecuteAction is expected to be called with one parameter
        expect(mockExecuteAction.mock.calls[0][0]).toMatchObject({
            name: "redeem",
            href: "/redeem",
        });
    });
});
