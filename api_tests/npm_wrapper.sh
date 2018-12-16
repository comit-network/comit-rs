#!/usr/bin/env bash

echo "Running $1";

export TEST_DIR="$1";

export NVM_SH=$([ -e $NVM_DIR/nvm.sh ] && \
    echo "$NVM_DIR/nvm.sh" || echo /usr/local/opt/nvm/nvm.sh );
. "$NVM_SH"
nvm use 11 || (nvm install 11 && nvm use 11);

cd "${ROOT}/api_tests";
yarn install;

npm test "./harness.js" \
    || { [ "$CAT_LOGS" ] && (cd "${TEST_DIR}/log"; tail -n +1 *.log); exit 1; }
