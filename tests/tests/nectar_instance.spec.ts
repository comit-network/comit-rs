import {
    parseBalanceOutput,
    parseDepositOutput,
} from "../src/environment/nectar_instance";

it("should parse the addresses from the output of the deposit command", () => {
    const output = `Bitcoin: bcrt1qfjfgjtyvm2sgxx2chluetnpwcpsvhpe6s4gh5d
Dai/Ether: 0x1b7075473088dd6ae749defd6d7060a17947e0d8`;

    const parsed = parseDepositOutput(output);

    expect(parsed.bitcoin).toEqual(
        "bcrt1qfjfgjtyvm2sgxx2chluetnpwcpsvhpe6s4gh5d"
    );
    expect(parsed.ethereum).toEqual(
        "0x1b7075473088dd6ae749defd6d7060a17947e0d8"
    );
});

it("should parse the balances from the output of the balance command", () => {
    const output = `Bitcoin: 10 BTC
Dai: 9000 DAI
Ether: 10 ETH`;

    const parsed = parseBalanceOutput(output);

    expect(parsed.bitcoin.toString()).toEqual(1_000_000_000n.toString());
    expect(parsed.dai.toString()).toEqual(
        9_000_000_000_000_000_000_000n.toString()
    );
    expect(parsed.ether.toString()).toEqual(
        10_000_000_000_000_000_000n.toString()
    );
});
