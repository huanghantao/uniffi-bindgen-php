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

## PHP Type Adapters

`[bindings.php.type_adapters.<TypeName>]` can widen PHP argument types and
provide a custom lowering expression for a UniFFI type. This is useful when a
package wants ergonomic domain-specific inputs while keeping the generator
generic.

```toml
[bindings.php.type_adapters.MyInterface]
type_hint = "string|MyInterface"
lower = "MyInterface::uniffiLower(MyInterfaceAdapter::from({value}))"
code = '''
final class MyInterfaceAdapter extends MyInterface
{
    // Adapter implementation appended to the generated PHP file.
}
'''
```

The `{value}` placeholder is replaced with the generated PHP argument
expression.
