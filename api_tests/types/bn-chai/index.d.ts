declare module "bn-chai" {
    function bnChaiFn(ctor: any): Chai.ChaiPlugin;

    export = bnChaiFn;
}

declare namespace Chai {
    interface NumberComparer {
        BN(value: any): void;
    }
}
