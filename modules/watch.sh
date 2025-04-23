#!/bin/sh

set -e

./regenerate.sh

cargo watch -s "./build-tail.sh"
