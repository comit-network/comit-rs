module.exports = {
    preset: "ts-jest",
    roots: ["<rootDir>/tests/e2e"],
    testRegex: "\\.ts$",
    transform: {
        "^.+\\.(t|j)s$": "ts-jest",
    },
    moduleFileExtensions: ["ts", "js", "json", "node"],
    testEnvironment: "<rootDir>/dist/src/e2e_test_environment",
    globalSetup: "<rootDir>/src/environment/prepare.ts",
    globalTeardown: "<rootDir>/src/environment/cleanup.ts",
    testTimeout: 63000,
};
