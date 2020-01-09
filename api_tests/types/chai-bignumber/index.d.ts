declare module "chai-bignumber" {
    function chaiBigNumberFn(): Chai.ChaiPlugin;

    export = chaiBigNumberFn;
}

declare namespace Chai {
    interface BigNumberComparison {
        equal: BigNumberComparison;
        equals: BigNumberComparison;
        eq: BigNumberComparison;
        above: BigNumberComparer;
        gt: BigNumberComparer;
        greaterThan: BigNumberComparer;
        least: BigNumberComparer;
        gte: BigNumberComparer;
        below: BigNumberComparer;
        lt: BigNumberComparer;
        lessThan: BigNumberComparer;
        most: BigNumberComparer;
        lte: BigNumberComparer;
    }

    interface BigNumberAssertion
        extends BigNumberComparison,
            BigNumberLanguageChain {
        finite: BigNumberAssertion;
        integer: BigNumberAssertion;
        negative: BigNumberAssertion;
        zero: BigNumberAssertion;
    }

    interface BigNumberLanguageChain {
        to: BigNumberAssertion;
        be: BigNumberAssertion;
        been: BigNumberAssertion;
        is: BigNumberAssertion;
        that: BigNumberAssertion;
        which: BigNumberAssertion;
        and: BigNumberAssertion;
        has: BigNumberAssertion;
        have: BigNumberAssertion;
        with: BigNumberAssertion;
        at: BigNumberAssertion;
        of: BigNumberAssertion;
        same: BigNumberAssertion;
        but: BigNumberAssertion;
        does: BigNumberAssertion;
    }

    interface LanguageChains {
        bignumber: BigNumberAssertion;
    }

    type BigNumberComparer = (value: any) => BigNumberAssertion;
}
