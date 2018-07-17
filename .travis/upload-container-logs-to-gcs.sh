#!/usr/bin/env bash

export GOOGLE_APPLICATION_CREDENTIALS="$TRAVIS_BUILD_DIR/.travis/gcloud-authentication.json"

# Setup google cloud SDK if it does not yet exist
gcloud version || true
if [ ! -d "$HOME/google-cloud-sdk/bin" ]; then rm -rf $HOME/google-cloud-sdk; export CLOUDSDK_CORE_DISABLE_PROMPTS=1; curl https://sdk.cloud.google.com | bash; fi
source $HOME/google-cloud-sdk/path.bash.inc
gcloud version

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
