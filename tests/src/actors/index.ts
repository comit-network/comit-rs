export type Role = "Alice" | "Bob";

export interface Stoppable {
    stop(): Promise<void>;
}

export interface DumpState {
    dumpState(): Promise<void>;
}
