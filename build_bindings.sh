#!/usr/bin/env bash
set -euo pipefail

cargo build -p text-fixture

LIB_PATH="$(find target/debug -maxdepth 1 \( -name 'libtext_fixture.dylib' -o -name 'libtext_fixture.so' -o -name 'text_fixture.dll' \) -print -quit)"
if [[ -z "${LIB_PATH}" ]]; then
  echo "Unable to find built text fixture cdylib under target/debug" >&2
  exit 1
fi

cargo run -p uniffi-bindgen-php -- generate \
  fixtures/text/src/text.udl \
  --crate text_fixture \
  --library "${LIB_PATH}" \
  --out-dir target/php-bindings
