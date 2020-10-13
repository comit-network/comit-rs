module.exports = {
    preset: "ts-jest",
    roots: ["<rootDir>/tests"],
    testRegex: "\\.ts$",
    transform: {
        "^.+\\.(t|j)s$": "ts-jest",
    },
    moduleFileExtensions: ["ts", "js", "json", "node"],
    testEnvironment: "<rootDir>/dist/src/environment/test_environment",
    globalSetup: "<rootDir>/src/environment/setup.ts",
    globalTeardown: "<rootDir>/src/environment/teardown.ts",
    setupFilesAfterEnv: [
        "<rootDir>/src/environment/jasmine_capture_current_testname.ts",
    ],
    testTimeout: 180000, // we are compiling cnd & nectar on demand, that can take quite a while sometimes
    bail: true,
};
