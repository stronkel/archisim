#!/usr/bin/env bash
# Builds ArchiSim for the browser (docs/IPAD.md): dist/web with the page,
# the program (WebAssembly) and the demo house, and dist/archisim-web.zip
# with the same, which the NAS downloads (scripts/nas/).
#
# Needs: rustup target add wasm32-unknown-unknown, and trunk
# (https://trunkrs.dev; it brings its own wasm-bindgen).
set -euo pipefail
cd "$(dirname "$0")/.."

rm -rf dist/web dist/archisim-web.zip
mkdir -p dist
trunk build --release --public-url ./ --dist dist/web crates/archisim-app/web/index.html

# The demo house, opened when the page starts.
cargo run --release -p archisim-cli -- demo out/web-demo
cp out/web-demo/demo.archisim dist/web/demo.archisim

# The version, for whoever serves it.
sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1 > dist/web/version.txt

ls -la dist/web
(cd dist/web && zip -qr ../archisim-web.zip .)
ls -la dist/archisim-web.zip
