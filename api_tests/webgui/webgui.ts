// These are tests related to embedding for comit-i in comit-rs
// They are system tests that checks that comit-i can be accessed and that comit-i can connect to cnd
// Functional tests on the GUI should only be ran in the comit-i repo!
import { expect, request, use } from "chai";
import chaiHttp = require("chai-http");
import "chai/register-should";
import { Actor } from "../lib/actor";
import { HarnessGlobal } from "../lib/util";

use(chaiHttp);

declare var global: HarnessGlobal;

const david = new Actor("david", global.config, global.project_root);

// the `setTimeout` forces it to be added on the event loop
// This is needed because there is no async call in the test
// And hence it does not get run without this `setTimeout`
setTimeout(async function() {
    describe("Web GUI tests", () => {
        it("Returns 403 'Forbidden for invalid origins or headers' for invalid preflight OPTIONS /swaps request for GET", async () => {
            const res = await request(david.cndHttpApiUrl())
                .options("/swaps")
                .set("Origin", "http://localhost:4000")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "GET");

            expect(res).to.have.status(403);
        });

        it("Returns 200 OK for preflight OPTIONS /swaps request for GET", async () => {
            const res = await request(david.cndHttpApiUrl())
                .options("/swaps")
                .set("Origin", "http://localhost:8080")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "GET");

            expect(res).to.have.status(200);
        });

        it("Returns 403 'Forbidden for invalid origins or headers' for invalid preflight OPTIONS /swaps/rfc003 request for POST", async () => {
            const res = await request(david.cndHttpApiUrl())
                .options("/swaps/rfc003")
                .set("Origin", "http://localhost:4000")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "POST");

            expect(res).to.have.status(403);
        });

        it("Returns 200 OK for preflight OPTIONS /swaps/rfc003 request for POST", async () => {
            const res = await request(david.cndHttpApiUrl())
                .options("/swaps/rfc003")
                .set("Origin", "http://localhost:8080")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "POST");

            expect(res).to.have.status(200);
        });

        it("[David] Sets appropriate CORS headers", async () => {
            const res = await request(david.cndHttpApiUrl())
                .get("/swaps")
                .set("Origin", "http://localhost:8080");

            expect(res).to.have.status(200);
            expect(res).to.have.header(
                "access-control-allow-origin",
                "http://localhost:8080"
            );
        });

        it("[David] Sets appropriate CORS headers on error responses", async () => {
            const res = await request(david.cndHttpApiUrl())
                .get("/swaps/rfc003/deadbeef-dead-beef-dead-deadbeefdead")
                .set("Origin", "http://localhost:8080");

            expect(res).to.have.status(404);
            expect(res).to.have.header(
                "access-control-allow-origin",
                "http://localhost:8080"
            );
        });

        it("[David] comit-i returns 200 OK", async () => {
            const res = await request(david.webGuiUrl()).get("/");

            expect(res).to.have.status(200);
        });

        it("[David] returns cnd http api settings on /config/cnd.js for GET", async function() {
            const res = await request(david.webGuiUrl())
                .get("/config/cnd.js?callback=callbackFunctionName")
                .set("Accept", "application/javascript")
                .buffer(true);

            expect(res).to.have.status(200);
            expect(res.text).to.match(/^function callbackFunctionName/);

            // tslint:disable-next-line:no-eval
            const fn = eval("(" + res.text + ")");
            const connDetails = fn();
            expect(connDetails).to.have.property("host", "127.0.0.1");
            expect(connDetails).to.have.property("port", 8123);
        });
    });

    run();
}, 0);
