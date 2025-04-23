#!/bin/sh

set -e

cargo build --target wasm32-unknown-unknown

# Generate a component for each of the output wasm files.
for path in target/wasm32-unknown-unknown/**/*.wasm; do
  echo "Generating component for $path"
  wasm-tools component new $path -o $path
done

echo "Done!"
