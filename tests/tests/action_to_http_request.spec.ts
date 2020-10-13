import axiosModule from "axios";
import nockModule from "nock";
import actionToHttpRequest from "../src/action_to_http_request";

describe("actionToHttpRequest", () => {
    it("given action with POST and no fields, should transform to correct http request", async () => {
        nock().post("/foo/bar").reply(200);

        const request = await actionToHttpRequest({
            name: "Foo bar action",
            method: "POST",
            href: "/foo/bar",
            type: "application/json",
        });
        const response = await axios().request(request);

        expect(response.status).toBe(200);
    });

    it("given action with POST with fields, should transform to correct http request", async () => {
        nock().post("/foo/bar", { aField: "some_value" }).reply(200);

        const request = await actionToHttpRequest(
            {
                name: "Foo bar action",
                method: "POST",
                href: "/foo/bar",
                type: "application/json",
                fields: [
                    {
                        name: "aField",
                        type: "text",
                    },
                ],
            },
            () => Promise.resolve("some_value")
        );

        const response = await axios().request(request);

        expect(response.status).toBe(200);
    });

    it("given action with unsupported target content-type, fails to serialize fields", async () => {
        const request = await actionToHttpRequest(
            {
                name: "Foo bar action",
                method: "POST",
                href: "/foo/bar",
                type: "application/x-www-form-urlencoded",
                fields: [
                    {
                        name: "aField",
                        type: "text",
                    },
                ],
            },
            () => Promise.resolve("some_value")
        );

        const response = axios().request(request);

        await expect(response).rejects.toThrowError(
            "Failed to serialize data for content-type application/x-www-form-urlencoded"
        );
    });

    it("given action with GET with fields, should transform to correct http request", async () => {
        nock().get("/foo/bar?aField=some_value").reply(200);

        const request = await actionToHttpRequest(
            {
                name: "Foo bar action",
                method: "GET",
                href: "/foo/bar",
                fields: [
                    {
                        name: "aField",
                        type: "text",
                    },
                ],
            },
            () => Promise.resolve("some_value")
        );

        const response = await axios().request(request);

        expect(response.status).toBe(200);
    });
});

function axios() {
    return axiosModule.create({
        baseURL: "http://example.com",
    });
}

function nock() {
    return nockModule("http://example.com");
}
