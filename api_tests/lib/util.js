{
    let _test_rng_counter = 0;

    function test_rng() {
        _test_rng_counter++;
        return Buffer.from(("" + _test_rng_counter).padStart(32, "0"));
    }

    module.exports.test_rng = test_rng;
}

{
    async function sleep(time) {
        return new Promise((res, rej) => {
            setTimeout(res, time);
        });
    }

    module.exports.sleep = sleep;
}
