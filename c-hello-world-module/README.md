# C module

These are rough notes of how I did make it work:

- Generated glue with:

```
wit-bindgen guest c -i ./wit/sys.wit -i ./wit/log.wit -d ./wit/trinity-module.wit --out-dir /tmp/c
```

- wrote the code in `main.c`. Probably full of memory leaks and footguns there. I can't C clearly anymore.
- Compiled with:

```
~/code/wasi-sdk/wasi-sdk-16.0/bin/clang \
    --sysroot ~/code/wasi-sdk/wasi-sdk-16.0/share/wasi-sysroot
    ./main.c
    ./trinity_module.c
    ./trinity_module_component_type.o
    -Wall -Wextra -Wno-unused-parameter
    -mexec-model=reactor
    -g
    -o out.wasm
```

- converted to a wasm component with:

```
wit-component ./out.wasm
```

- moved to the watched directory of trinity:

```
cp out.wasm ../modules/target/wasm32-unknown-unknown/release
```
