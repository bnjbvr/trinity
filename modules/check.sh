#!/bin/sh

set -e
./regenerate.sh
cargo check --target wasm32-unknown-unknown
echo "Done!"
