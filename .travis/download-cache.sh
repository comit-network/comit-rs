#!/usr/bin/env bash

rsync -haz --stats -e "ssh -i $TRAVIS_BUILD_DIR/.travis/travis_build_cache -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null" travis@35.201.25.160:/home/travis/.cache/sccache $HOME/.cache/
chmod 777 -R $HOME/.cache/sccache
