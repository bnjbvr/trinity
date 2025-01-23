#!/bin/sh

set -e

cargo watch -x "component build --target wasm32-unknown-unknown"
