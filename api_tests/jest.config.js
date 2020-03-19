module.exports = {
    preset: "ts-jest",
    roots: ["<rootDir>/tests"],
    testRegex: "\\.ts$",
    transform: {
        "^.+\\.(t|j)s$": "ts-jest",
    },
    moduleFileExtensions: ["ts", "js", "json", "node"],
    testEnvironment: "<rootDir>/dist/src/test_environment",
    globalSetup: "<rootDir>/src/environment/cleanup.ts",
    globalTeardown: "<rootDir>/src/environment/cleanup.ts",
    testTimeout: 123000,
};
