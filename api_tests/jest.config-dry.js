module.exports = {
    preset: "ts-jest",
    roots: ["<rootDir>/tests/dry"],
    testRegex: "\\.ts$",
    transform: {
        "^.+\\.(t|j)s$": "ts-jest",
    },
    moduleFileExtensions: ["ts", "js", "json", "node"],
    testEnvironment: "<rootDir>/dist/src/dry_test_environment",
    testTimeout: 63000,
    setupFilesAfterEnv: ["<rootDir>/src/configure_jasmine.ts"],
};
