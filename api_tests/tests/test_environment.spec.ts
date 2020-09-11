import { extractCndConfigOverrides } from "../src/environment/test_environment";

describe("extractCndConfigOverrides", () => {
    test("given no overrides, returns empty object", () => {
        expect(extractCndConfigOverrides({})).toStrictEqual({});
        expect(
            extractCndConfigOverrides({
                cndConfigOverride: null,
            })
        ).toStrictEqual({});
        expect(
            extractCndConfigOverrides({
                cndConfigOverride: "",
            })
        ).toStrictEqual({});
        expect(
            extractCndConfigOverrides({
                cndConfigOverride: ["", "", ""],
            })
        ).toStrictEqual({});
    });

    test("given overrides, sets nested key", () => {
        const overrides = {
            cndConfigOverride:
                "ethereum.tokens.dai = 0x0000000000000000000000000000000000000000",
        };

        const config = extractCndConfigOverrides(overrides);

        expect(config).toStrictEqual({
            ethereum: {
                tokens: {
                    dai: "0x0000000000000000000000000000000000000000",
                },
            },
        });
    });

    test("given two overrides, sets them both", () => {
        const overrides = {
            cndConfigOverride: [
                "ethereum.tokens.dai = 0x0000000000000000000000000000000000000000",
                "bitcoin.network = regtest",
            ],
        };

        const config = extractCndConfigOverrides(overrides);

        expect(config).toStrictEqual({
            ethereum: {
                tokens: {
                    dai: "0x0000000000000000000000000000000000000000",
                },
            },
            bitcoin: {
                network: "regtest",
            },
        });
    });
});
