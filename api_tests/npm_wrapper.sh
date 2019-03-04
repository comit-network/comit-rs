#!/usr/bin/env bash

echo "Running $1";

export TEST_DIR="$1";

cd "${ROOT}/api_tests";
yarn install;

npm test "./harness.js" \
    || { [ "$CAT_LOGS" ] && (cd "${TEST_DIR}/log"; tail -n +1 *.log); exit 1; }
