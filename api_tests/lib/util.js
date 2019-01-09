{
    let _test_rng_counter = 0;

    function test_rng() {
        _test_rng_counter++;
        return Buffer.from(("" + _test_rng_counter).padStart(32, "0"));
    }

    module.exports.test_rng = test_rng;
}

{
    const logger = global.harness.logger;
    module.exports.logger = function() {
        return logger;
    };
}

{
    async function sleep(time) {
        return new Promise((res, rej) => {
            setTimeout(res, time);
        });
    }

    module.exports.sleep = sleep;
}

{
    function project_root() {
        return global.harness.project_root;
    }

    module.exports.project_root = project_root();
}
