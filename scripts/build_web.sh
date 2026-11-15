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
# Smaller: the whole program optimised at once, without symbols (and
# wasm-opt, in index.html).
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_STRIP=true
# Optimised for size: a third less to load on the iPad (the render goes in
# the workers, a little slower).
export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
# Trunk finds the program's package from the folder it is run in.
(cd crates/archisim-app && trunk build --release --public-url ./ --dist "$PWD/../../dist/web" index.html)

# The demo house, opened when the page starts (built as usual, not as
# the web program).
unset CARGO_PROFILE_RELEASE_LTO CARGO_PROFILE_RELEASE_CODEGEN_UNITS CARGO_PROFILE_RELEASE_STRIP CARGO_PROFILE_RELEASE_OPT_LEVEL
cargo run --release -p archisim-cli -- demo out/web-demo
cp out/web-demo/demo.archisim dist/web/demo.archisim

# The version, for whoever serves it.
sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1 > dist/web/version.txt

ls -la dist/web
# How big the program is, as it goes and as it travels (gzip).
for f in dist/web/*.wasm; do
  echo "programa: $(du -h "$f" | cut -f1), comprimido: $(gzip -9c "$f" | wc -c | awk '{printf "%.1f MB", $1/1048576}')"
done
(cd dist/web && zip -qr ../archisim-web.zip .)
ls -la dist/archisim-web.zip
