import { Problem } from "../src/axios_rfc7807_middleware";

describe("problem message", () => {
    it("should format all fields to a descriptive message", () => {
        const problem = new Problem({
            title: "Swap not supported.",
            status: 400,
            detail: "This combination of protocols is not supported.",
            type: "https://comit.network/docs/errors/swap-not-supported",
        });

        expect(problem.message).toMatchInlineSnapshot(
            `"Request failed with status code 400: Swap not supported. This combination of protocols is not supported. See https://comit.network/docs/errors/swap-not-supported for more information."`
        );
    });

    it("should exclude the url if it is not present", () => {
        const problem = new Problem({
            title: "Swap not supported.",
            status: 400,
            detail: "This combination of protocols is not supported.",
        });

        expect(problem.message).toMatchInlineSnapshot(
            `"Request failed with status code 400: Swap not supported. This combination of protocols is not supported."`
        );
    });

    it("should exclude the statuscode if it is not present", () => {
        const problem = new Problem({
            title: "Swap not supported.",
            detail: "This combination of protocols is not supported.",
        });

        expect(problem.message).toMatchInlineSnapshot(
            `"Request failed: Swap not supported. This combination of protocols is not supported."`
        );
    });

    it("should exclude the detail message if it is not present", () => {
        const problem = new Problem({
            title: "Swap not supported.",
        });

        expect(problem.message).toMatchInlineSnapshot(
            `"Request failed: Swap not supported."`
        );
    });
});
