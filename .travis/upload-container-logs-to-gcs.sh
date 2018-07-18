#!/usr/bin/env bash

gcloud auth activate-service-account --key-file="$TRAVIS_BUILD_DIR/.travis/gcloud-authentication.json"

LOG_DIR="build_$TRAVIS_COMMIT"
LOG_DIR_PATH="/tmp/$LOG_DIR"

mkdir -p $LOG_DIR_PATH
for container in $(docker ps -aq)
do
    docker logs ${container} > "$LOG_DIR_PATH/$container.log"
done

ARCHIVE_NAME="$LOG_DIR.tar"

cd /tmp
tar -czvf $ARCHIVE_NAME $LOG_DIR
gsutil cp $ARCHIVE_NAME gs://swap-log-storage
