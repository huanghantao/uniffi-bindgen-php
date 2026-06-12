# uniffi-bindgen-php

Experimental PHP bindings generator for UniFFI components.

This repository contains a standalone `uniffi-bindgen-php` binary. It uses the
remote UniFFI repository as a Cargo git dependency, not a local checkout.

## Build

```sh
./build.sh
```

## Generate the fixture bindings

```sh
./build_bindings.sh
```

The generated PHP file is written to `target/php-bindings`.

