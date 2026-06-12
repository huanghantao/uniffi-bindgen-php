pub mod gen_php;

pub use gen_php::{
    generate_php_bindings, generate_php_bindings_from_library, Config, PhpBindingGenerator,
};
