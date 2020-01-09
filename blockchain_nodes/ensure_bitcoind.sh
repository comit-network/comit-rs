#!/bin/bash
set -e

case "$OSTYPE" in
  darwin*)  OS="OSX"; url="https://bitcoincore.org/bin/bitcoin-core-0.17.0/bitcoin-0.17.0-osx64.tar.gz";;
  linux*)   OS="Linux"; url="https://bitcoincore.org/bin/bitcoin-core-0.17.0/bitcoin-0.17.0-x86_64-linux-gnu.tar.gz";;
  *)        echo "unknown: $OSTYPE. Sorry, this is currently not supported"; exit 1 ;;
esac


TARGET_FOLDER=./blockchain_nodes/bitcoin

if [ ! -d "${TARGET_FOLDER}" ]
then
    mkdir -p "${TARGET_FOLDER}"
fi

TARGET_FILE="${TARGET_FOLDER}/bitcoin-0.17.0"
if [ -d "$TARGET_FILE" ]; then
  exit 0;
fi


echo "Downloading bitcoind for ${OS}";
curl -s "$url" | tar xvz -C $TARGET_FOLDER
chmod +x "${TARGET_FILE}/bin/bitcoind"
