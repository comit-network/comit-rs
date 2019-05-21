// These are tests related to embedding for comit-i in comit-rs
// They are system tests that checks that comit-i can be accessed and that comit-i can connect to comit_node
// Functional tests on the GUI should only be ran in the comit-i repo!
import { Actor } from "../lib/actor";
import { use, request, expect } from "chai";
import { HarnessGlobal } from "../lib/util";
import "chai/register-should";
import chaiHttp = require("chai-http");

use(chaiHttp);

declare var global: HarnessGlobal;

const david = new Actor("david", global.config, global.project_root);

// the `setTimeout` forces it to be added on the event loop
// This is needed because there is no async call in the test
// And hence it does not get run without this `setTimeout`
setTimeout(async function() {
    describe("Web GUI tests", () => {
        it("Returns 403 'Forbidden for invalid origins or headers' for invalid preflight OPTIONS /swaps request for GET", async () => {
            let res = await request(david.comit_node_url())
                .options("/swaps")
                .set("Origin", "http://localhost:4000")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "GET");

            res.should.have.status(403);
        });

        it("Returns 200 OK for preflight OPTIONS /swaps request for GET", async () => {
            let res = await request(david.comit_node_url())
                .options("/swaps")
                .set("Origin", "http://localhost:8080")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "GET");

            res.should.have.status(200);
        });

        it("Returns 403 'Forbidden for invalid origins or headers' for invalid preflight OPTIONS /swaps/rfc003 request for POST", async () => {
            let res = await request(david.comit_node_url())
                .options("/swaps/rfc003")
                .set("Origin", "http://localhost:4000")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "POST");

            res.should.have.status(403);
        });

        it("Returns 200 OK for preflight OPTIONS /swaps/rfc003 request for POST", async () => {
            let res = await request(david.comit_node_url())
                .options("/swaps/rfc003")
                .set("Origin", "http://localhost:8080")
                .set("Access-Control-Request-Headers", "content-type")
                .set("Access-Control-Request-Method", "POST");

            res.should.have.status(200);
        });

        it("[David] Sets appropriate CORS headers", async () => {
            let res = await request(david.comit_node_url())
                .get("/swaps")
                .set("Origin", "http://localhost:8080");

            res.should.have.status(200);
            res.should.have.header(
                "access-control-allow-origin",
                "http://localhost:8080"
            );
        });

        it("[David] comit-i returns 200 OK", async () => {
            let res = await request(david.web_gui_url()).get("/");

            res.should.have.status(200);
        });

        it("[David] returns comit_node http api settings on /config/comit_node.js for GET", async function() {
            let res = await request(david.web_gui_url())
                .get("/config/comitNode.js?callback=callbackFunctionName")
                .set("Accept", "application/javascript")
                .buffer(true);

            res.should.have.status(200);
            const body: string = res.text;
            expect(body).to.match(/^function callbackFunctionName/);

            let fn = eval("(" + body + ")");
            let connDetails = fn();
            expect(connDetails).to.have.property("host", "127.0.0.1");
            expect(connDetails).to.have.property("port", 8000);
        });
    });

    run();
}, 0);
