#!/usr/bin/env sh

wasm-pack build --target web
python3 -m http.server
