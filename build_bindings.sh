#!/bin/bash
set -euxo pipefail

SCRIPT_DIR="${SCRIPT_DIR:-$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )}"
ROOT_DIR="$SCRIPT_DIR"

BINDINGS_DIR="$ROOT_DIR/out"
BINARIES_DIR="$ROOT_DIR/target/debug"

rm -rf $BINDINGS_DIR
mkdir $BINDINGS_DIR

# FIXME: It would be better to generate and build fixtures one by one, instead of combining
# them all into the same library

if [[ "$OSTYPE" == "darwin"* ]]; then
LIB_FILE="$BINARIES_DIR/libuniffi_fixtures.dylib"
else 
LIB_FILE="$BINARIES_DIR/libuniffi_fixtures.so"
fi
target/debug/uniffi-bindgen-php $LIB_FILE --out-dir "$BINDINGS_DIR" --library --config "$ROOT_DIR/fixtures/uniffi.toml"
