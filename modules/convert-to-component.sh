#!/bin/sh

for path in target/wasm32-unknown-unknown/**/*.wasm; do
  echo "Generating component for $path"
  wasm-tools component new $path -o $path
done
