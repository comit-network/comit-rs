#!/bin/bash

function navigate_to_last_folder() {
  cd $(ls -d */|tail -n 1)
}

RELEASE_TAG=$(git tag | tail -1)
RELEASE_VERSION=$(echo $RELEASE_TAG | cut -d'-' -f 1)
echo "Checking for current release version ${RELEASE_VERSION}"

# navigate to latest migration
cd migrations
MIGRATIONS_FOLDER=$(pwd)
navigate_to_last_folder
LATEST_MIGRATION=$(pwd)
# get the latest commit of the latest migration
LATEST_COMMIT=$(git log -n 1 --pretty=format:%H -- $LATEST_MIGRATION)

# check if the commit yields any tags
TAG_LIST=$(git tag --contains $LATEST_COMMIT)

if [[ $TAG_LIST == *"${RELEASE_VERSION}"* ]]; then
  echo "A new migration is needed, please enter a name for the new migration based on your planned changes:"
  echo "Naming according to Diesel conventions, separating words with '_'."
  read MIGRATION_NAME
  cd $MIGRATIONS_FOLDER
  which diesel || cargo install diesel_cli --no-default-features --features sqlite
  diesel migration generate $MIGRATION_NAME
  navigate_to_last_folder
  LATEST_MIGRATION=$(pwd)
else
  echo "No new migration needed."
fi

echo "Use this migration for your changes: ${LATEST_MIGRATION}"
