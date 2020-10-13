/**
 * The FakeTreasuryService implements the same API as the Kraken ticker.
 *
 * Nectar uses the Kraken ticker to publish orders with a spread.
 * By pointing nectar to an instance of the FakeTreasureService, we can control the rate and allow our test suite to
 * not require Internet access.
 */

import express from "express";
import morgan from "morgan";

const price = process.argv[2];
const port = process.argv[3];

// tslint:disable-next-line:no-floating-promises
run(parseInt(price, 10), parseInt(port, 10));

async function run(btcDaiPrice: number, port: number) {
    const app = express();

    app.use(morgan("tiny"));
    app.get("/0/public/Ticker", (req, res) => {
        if (req.query.pair !== "XBTDAI") {
            res.status(400);
            return;
        }

        res.status(200).json({
            error: [],
            result: {
                XBTDAI: {
                    a: [`${btcDaiPrice}.00000`, "1", "1.000"],
                    b: [`${btcDaiPrice}.00000`, "1", "1.000"],
                },
            },
        });
    });

    app.listen(port);
}
