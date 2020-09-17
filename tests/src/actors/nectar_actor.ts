import { DumpState, GetListenAddress, GetPeerId, Stoppable } from "./index";
import { Logger } from "log4js";
import { Balances, NectarInstance } from "../environment/nectar_instance";

export default class NectarActor
    implements Stoppable, DumpState, GetPeerId, GetListenAddress {
    private startingBalances: Balances;

    constructor(
        public readonly logger: Logger,
        public readonly nectarInstance: NectarInstance,
        private readonly peerId: string
    ) {
        logger.info(
            "Created new nectar actor with config",
            nectarInstance.config
        );
    }

    async dumpState(): Promise<void> {
        return Promise.resolve(undefined);
    }

    async stop(): Promise<void> {
        return this.nectarInstance.stop();
    }

    async getListenAddress(): Promise<string> {
        // Very dirty hack that assumes we are listening on an IPv4 interface :crossed_fingers:
        const address = this.nectarInstance.config.network.listen[0].replace(
            "0.0.0.0",
            "127.0.0.1"
        );

        return Promise.resolve(address);
    }

    async getPeerId(): Promise<string> {
        return Promise.resolve(this.peerId);
    }

    public async saveBalancesBeforeSwap() {
        this.startingBalances = await this.nectarInstance.balance();
    }

    public async assertBalancesChangedBy(expectedDiff: Partial<Balances>) {
        const newBalances = await this.nectarInstance.balance();

        if (expectedDiff.bitcoin) {
            const expectedBalance =
                this.startingBalances.bitcoin + expectedDiff.bitcoin;

            expect(newBalances.bitcoin.toString()).toEqual(
                expectedBalance.toString()
            );
        }
        if (expectedDiff.ether) {
            const expectedBalance =
                this.startingBalances.ether + expectedDiff.ether;

            expect(newBalances.ether.toString()).toEqual(
                expectedBalance.toString()
            );
        }
        if (expectedDiff.dai) {
            const expectedBalance =
                this.startingBalances.dai + expectedDiff.dai;

            expect(newBalances.dai.toString()).toEqual(
                expectedBalance.toString()
            );
        }
    }
}
