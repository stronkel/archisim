#!/usr/bin/env bash
# Downloads buildingSMART's IDS test cases (licence CC BY-ND 4.0, kept
# outside the repository) and runs ArchiSim's checker against all of them.
#
#   scripts/ids_testcases.sh [folder]     (default: /tmp/ids-testcases)
set -euo pipefail
dest="${1:-/tmp/ids-testcases}"
if [ ! -d "$dest/IDS" ]; then
  mkdir -p "$dest"
  git clone --depth 1 --filter=blob:none --sparse https://github.com/buildingSMART/IDS.git "$dest/IDS"
  git -C "$dest/IDS" sparse-checkout set Documentation/ImplementersDocumentation/TestCases
fi
IDS_TESTCASES="$dest/IDS/Documentation/ImplementersDocumentation/TestCases" \
  cargo test -p archisim-ids --test conformance -- --nocapture
