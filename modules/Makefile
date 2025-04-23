.DEFAULT_GOAL := help

build-tail:
	cargo build --target wasm32-unknown-unknown
	./convert-to-component.sh

install-tools: ## Install all the tools necessary to build the components.
	# Keep those values in sync with the main CI workflow!
	cargo install --locked wit-bindgen-cli@0.41.0 --force
	cargo install --locked wasm-tools@1.229.0 --force

bindings: ## Regenerate the Rust bindings from the WIT files, for the libraries.
	# Generate Rust bindings for each library.
	wit-bindgen rust ../wit/kv.wit --out-dir wit-kv/src/ --format --runtime-path wit_bindgen_rt
	wit-bindgen rust ../wit/log.wit --out-dir wit-log/src/ --format --runtime-path wit_bindgen_rt
	wit-bindgen rust ../wit/sync-request.wit --out-dir wit-sync-request/src/ --format --runtime-path wit_bindgen_rt
	wit-bindgen rust ../wit/sys.wit --out-dir wit-sys/src/ --format --runtime-path wit_bindgen_rt
	
	# Generate Rust bindings for the export library.
	wit-bindgen rust ../wit/trinity-module.wit --out-dir libcommand/src/ --format --runtime-path wit_bindgen_rt --pub-export-macro

build: bindings build-tail ## Build all the component modules in debug mode.
	@echo

check: bindings ## Check all the component modules in debug mode.
	cargo check --target wasm32-unknown-unknown

watch: bindings ## Regenerates bindings once and watches for changes in the Rust source files.
	cargo watch -s "./build-tail.sh"

clean: ## Gets rid of the directory target.
	cargo clean

help: ## Show the help.
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
