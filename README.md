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

## PHP Object Extensions

`[bindings.php.object_extensions.<ObjectName>]` can append PHP methods to a
generated object class. This is useful for package-specific convenience APIs
that should live on the generated object instead of in a separate static helper
class.

The key can be the UniFFI object name or the generated PHP class name. The
`code` block is inserted verbatim inside the generated PHP class body, just
before the closing brace.

```toml
[bindings.php.object_extensions.MyMap]
code = '''
    public function set(string $key, mixed $value): void
    {
        $this->insert($key, $value);
    }

    public function toJSON(): mixed
    {
        return MyValueAdapter::toPhp($this->getDeepValue());
    }
'''
```

Object extensions are best for adding new instance methods that compose
generated methods. They should not duplicate an existing generated method name,
because PHP does not support method overloading.

Type adapters and object extensions can be used together: type adapters change
generated method argument hints and lowering expressions, while object
extensions add package-level instance methods to generated classes.
