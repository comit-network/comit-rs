declare namespace jasmine {
    let currentTestName: string | undefined;

    // Necessary types copied from: https://github.com/DefinitelyTyped/DefinitelyTyped/blob/master/types/jasmine/ts3.1/index.d.ts

    function getEnv(): Env;

    interface Env {
        addReporter(reporter: CustomReporter): void;
    }

    interface CustomReporter {
        specStarted?(result: CustomReporterResult): void;
    }

    interface CustomReporterResult {
        description: string;
    }
}
