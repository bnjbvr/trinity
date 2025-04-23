#!/bin/sh

set -e

# Generate Rust bindings for each library.
wit-bindgen rust ../wit/kv.wit --out-dir wit-kv/src/ --format --runtime-path wit_bindgen_rt
wit-bindgen rust ../wit/log.wit --out-dir wit-log/src/ --format --runtime-path wit_bindgen_rt
wit-bindgen rust ../wit/sync-request.wit --out-dir wit-sync-request/src/ --format --runtime-path wit_bindgen_rt
wit-bindgen rust ../wit/sys.wit --out-dir wit-sys/src/ --format --runtime-path wit_bindgen_rt

# Generate Rust bindings for the export library.
wit-bindgen rust ../wit/trinity-module.wit --out-dir libcommand/src/ --format --runtime-path wit_bindgen_rt --pub-export-macro
