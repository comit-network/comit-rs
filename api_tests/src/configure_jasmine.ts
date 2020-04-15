jasmine.getEnv().addReporter({
    specStarted: (result) =>
        // @ts-ignore
        (jasmine.currentTestName = result.description),
});
