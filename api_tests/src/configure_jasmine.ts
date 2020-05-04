// @ts-ignore: Jasmine types are not up to date
jasmine.getEnv().addReporter({
    specStarted: (result: any) =>
        // @ts-ignore
        (jasmine.currentTestName = result.description),
});
