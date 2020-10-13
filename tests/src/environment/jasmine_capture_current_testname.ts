import CustomReporterResult = jasmine.CustomReporterResult;

jasmine.getEnv().addReporter({
    specStarted: (result: CustomReporterResult) =>
        (jasmine.currentTestName = result.description),
});
