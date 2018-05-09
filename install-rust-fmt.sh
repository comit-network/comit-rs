#!/bin/sh

rustup toolchain install stable-2018-03-29
rustup component add rustfmt-preview --toolchain=stable-2018-03-29

touch .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit

echo "#!/bin/bash
set -eu

cargo +stable-2018-03-29 fmt -- --write-mode diff" > .git/hooks/pre-commit