#!/bin/bash
set -e


case "$OSTYPE" in
  darwin*)  OS="OSX"; url="https://releases.parity.io/ethereum/v2.7.2/x86_64-apple-darwin/parity";;
  linux*)   OS="Linux"; url="https://releases.parity.io/ethereum/v2.7.2/x86_64-unknown-linux-gnu/parity";;
  *)        echo "unknown: $OSTYPE. Sorry, this is currently not supported"; exit 1 ;;
esac

TARGET_FOLDER=./blockchain_nodes/parity

if [ ! -d "${TARGET_FOLDER}" ]
then
    mkdir -p "${TARGET_FOLDER}"
fi

TARGET_FILE="${TARGET_FOLDER}/parity"
if [ -f "$TARGET_FILE" ]; then
  exit 0;
fi

echo "Downloading parity for ${OS}";
curl -s "$url" -o $TARGET_FILE
chmod +x $TARGET_FILE
