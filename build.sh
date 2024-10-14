#!/bin/bash
set -euxo pipefail

cargo build --package uniffi-bindgen-php --package uniffi-bindgen-php-fixtures
