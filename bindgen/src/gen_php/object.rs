use std::collections::HashMap;

use anyhow::{bail, Result};
use heck::{ToLowerCamelCase, ToUpperCamelCase};
use uniffi_bindgen::interface::{Argument, AsType, Callable, Object, ObjectImpl, Type};

use super::PhpTypeAdapter;

pub(crate) type PhpTypeAdapters = HashMap<String, PhpTypeAdapter>;

#[derive(Debug, Clone)]
pub(crate) struct PhpArg {
    pub(crate) decl: String,
    pub(crate) lower_expr: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PhpCallable {
    pub(crate) name: String,
    pub(crate) ffi_name: String,
    pub(crate) throws_type_spec: String,
    pub(crate) args_decl: String,
    pub(crate) args_call: String,
    pub(crate) return_type_decl: String,
    pub(crate) has_return: bool,
    pub(crate) lift_expr: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PhpObject {
    pub(crate) class_name: String,
    pub(crate) proxy_class_name: String,
    pub(crate) free_ffi_name: String,
    pub(crate) clone_ffi_name: String,
    pub(crate) is_callback_trait: bool,
    pub(crate) primary_constructor: Option<PhpCallable>,
    pub(crate) alternate_constructors: Vec<PhpCallable>,
    pub(crate) methods: Vec<PhpCallable>,
}

pub(crate) fn build_function(
    name: &str,
    callable: &impl Callable,
    type_adapters: &PhpTypeAdapters,
) -> Result<PhpCallable> {
    build_callable(name, callable, CallableKind::Function, type_adapters)
}

pub(crate) fn build_method(
    name: &str,
    callable: &impl Callable,
    type_adapters: &PhpTypeAdapters,
) -> Result<PhpCallable> {
    build_callable(name, callable, CallableKind::Method, type_adapters)
}

pub(crate) fn build_constructor(
    name: &str,
    callable: &impl Callable,
    type_adapters: &PhpTypeAdapters,
) -> Result<PhpCallable> {
    build_callable(name, callable, CallableKind::Constructor, type_adapters)
}

pub(crate) fn build_object(obj: &Object, type_adapters: &PhpTypeAdapters) -> Result<PhpObject> {
    let primary_constructor = obj
        .primary_constructor()
        .map(|c| build_constructor(c.name(), c, type_adapters))
        .transpose()?;

    let alternate_constructors = obj
        .alternate_constructors()
        .into_iter()
        .map(|c| build_constructor(c.name(), c, type_adapters))
        .collect::<Result<Vec<_>>>()?;

    let methods = obj
        .methods()
        .into_iter()
        .map(|m| build_method(m.name(), m, type_adapters))
        .collect::<Result<Vec<_>>>()?;

    Ok(PhpObject {
        class_name: class_name(obj.name()),
        proxy_class_name: format!("UniFFI{}Proxy", class_name(obj.name())),
        free_ffi_name: obj.ffi_object_free().name().to_string(),
        clone_ffi_name: obj.ffi_object_clone().name().to_string(),
        is_callback_trait: obj.has_callback_interface(),
        primary_constructor,
        alternate_constructors,
        methods,
    })
}

#[derive(Debug, Copy, Clone)]
enum CallableKind {
    Function,
    Method,
    Constructor,
}

fn build_callable(
    name: &str,
    callable: &impl Callable,
    kind: CallableKind,
    type_adapters: &PhpTypeAdapters,
) -> Result<PhpCallable> {
    if callable.is_async() {
        bail!("PHP bindings do not support async callable `{name}` yet");
    }

    let args = callable
        .arguments()
        .into_iter()
        .map(|arg| build_arg(&arg, type_adapters))
        .collect::<Result<Vec<_>>>()?;
    let args_decl = args
        .iter()
        .map(|arg| arg.decl.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let args_call = args
        .iter()
        .map(|arg| arg.lower_expr.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let (return_type_decl, has_return, lift_expr) = match (kind, callable.return_type()) {
        (CallableKind::Constructor, _) => (String::new(), true, String::new()),
        (_, Some(return_type)) => (
            format!(": {}", php_type_hint(return_type)?),
            true,
            lift_expr("$result", return_type)?,
        ),
        (_, None) => (": void".to_string(), false, String::new()),
    };
    let throws_type_spec = callable
        .throws_type()
        .map(type_spec)
        .unwrap_or_else(String::new);

    Ok(PhpCallable {
        name: function_name(name),
        ffi_name: callable.ffi_func().name().to_string(),
        throws_type_spec,
        args_decl,
        args_call,
        return_type_decl,
        has_return,
        lift_expr,
    })
}

fn build_arg(arg: &Argument, type_adapters: &PhpTypeAdapters) -> Result<PhpArg> {
    if arg.default_value().is_some() {
        bail!(
            "PHP bindings do not support default argument values yet: `{}`",
            arg.name()
        );
    }
    let name = variable_name(arg.name());
    let ty = arg.as_type();
    Ok(PhpArg {
        decl: format!("{} ${name}", php_arg_type_hint(&ty, type_adapters)?),
        lower_expr: lower_arg_expr(&format!("${name}"), &ty, type_adapters)?,
    })
}

pub(crate) fn validate_type(type_: &Type) -> Result<()> {
    match type_ {
        Type::UInt8
        | Type::Int8
        | Type::UInt16
        | Type::Int16
        | Type::UInt32
        | Type::Int32
        | Type::UInt64
        | Type::Int64
        | Type::Float32
        | Type::Float64
        | Type::Boolean
        | Type::String
        | Type::Bytes
        | Type::Object { .. }
        | Type::Record { .. }
        | Type::Enum { .. }
        | Type::CallbackInterface { .. }
        | Type::Optional { .. }
        | Type::Sequence { .. }
        | Type::Map { .. }
        | Type::Timestamp
        | Type::Duration
        | Type::Custom { .. } => Ok(()),
    }
}

pub(crate) fn php_type_hint(type_: &Type) -> Result<String> {
    Ok(match type_ {
        Type::UInt8
        | Type::Int8
        | Type::UInt16
        | Type::Int16
        | Type::UInt32
        | Type::Int32
        | Type::Int64 => "int".to_string(),
        Type::UInt64 => "int|string".to_string(),
        Type::Float32 | Type::Float64 => "float".to_string(),
        Type::Boolean => "bool".to_string(),
        Type::String | Type::Bytes => "string".to_string(),
        Type::Object { name, .. } | Type::Record { name, .. } | Type::Enum { name, .. } => {
            class_name(name)
        }
        Type::CallbackInterface { name, .. } => class_name(name),
        Type::Optional { inner_type } => {
            let inner = php_type_hint(inner_type)?;
            if inner == "mixed" {
                "mixed".to_string()
            } else if inner.contains('|') {
                format!("{inner}|null")
            } else {
                format!("?{inner}")
            }
        }
        Type::Sequence { .. } | Type::Map { .. } => "array".to_string(),
        Type::Timestamp | Type::Duration | Type::Custom { .. } => "mixed".to_string(),
    })
}

fn php_arg_type_hint(type_: &Type, type_adapters: &PhpTypeAdapters) -> Result<String> {
    Ok(match type_adapter(type_, type_adapters) {
        Some(adapter) => adapter.type_hint.clone(),
        None => php_type_hint(type_)?,
    })
}

pub(crate) fn lower_expr(value: &str, type_: &Type) -> Result<String> {
    Ok(match type_ {
        Type::UInt8 => format!("UniFFIRuntime::lowerUInt8({value})"),
        Type::Int8 => format!("UniFFIRuntime::lowerInt8({value})"),
        Type::UInt16 => format!("UniFFIRuntime::lowerUInt16({value})"),
        Type::Int16 => format!("UniFFIRuntime::lowerInt16({value})"),
        Type::UInt32 => format!("UniFFIRuntime::lowerUInt32({value})"),
        Type::Int32 => format!("UniFFIRuntime::lowerInt32({value})"),
        Type::UInt64 => format!("UniFFIRuntime::lowerUInt64({value})"),
        Type::Int64 => format!("UniFFIRuntime::lowerInt64({value})"),
        Type::Float32 => format!("UniFFIRuntime::lowerFloat32({value})"),
        Type::Float64 => format!("UniFFIRuntime::lowerFloat64({value})"),
        Type::Boolean => format!("UniFFIRuntime::lowerBool({value})"),
        Type::String => format!("UniFFIRuntime::lowerString({value})"),
        Type::Bytes => format!("UniFFIRuntime::lowerBytes({value})"),
        Type::Object { name, .. }
        | Type::Record { name, .. }
        | Type::Enum { name, .. }
        | Type::CallbackInterface { name, .. } => {
            format!("{}::uniffiLower({value})", class_name(name))
        }
        Type::Optional { .. }
        | Type::Sequence { .. }
        | Type::Map { .. }
        | Type::Timestamp
        | Type::Duration
        | Type::Custom { .. } => {
            format!(
                "UniFFIRuntime::lowerSerialized({value}, {})",
                type_spec(type_)
            )
        }
    })
}

fn lower_arg_expr(value: &str, type_: &Type, type_adapters: &PhpTypeAdapters) -> Result<String> {
    if let Some(adapter) = type_adapter(type_, type_adapters) {
        if !adapter.lower.contains("{value}") {
            bail!("PHP type adapter lower expression must contain `{{value}}`");
        }

        return Ok(adapter.lower.replace("{value}", value));
    }

    lower_expr(value, type_)
}

pub(crate) fn lift_expr(value: &str, type_: &Type) -> Result<String> {
    Ok(match type_ {
        Type::UInt8 => format!("UniFFIRuntime::liftUInt8({value})"),
        Type::Int8 => format!("UniFFIRuntime::liftInt8({value})"),
        Type::UInt16 => format!("UniFFIRuntime::liftUInt16({value})"),
        Type::Int16 => format!("UniFFIRuntime::liftInt16({value})"),
        Type::UInt32 => format!("UniFFIRuntime::liftUInt32({value})"),
        Type::Int32 => format!("UniFFIRuntime::liftInt32({value})"),
        Type::UInt64 => format!("UniFFIRuntime::liftUInt64({value})"),
        Type::Int64 => format!("UniFFIRuntime::liftInt64({value})"),
        Type::Float32 => format!("UniFFIRuntime::liftFloat32({value})"),
        Type::Float64 => format!("UniFFIRuntime::liftFloat64({value})"),
        Type::Boolean => format!("UniFFIRuntime::liftBool({value})"),
        Type::String => format!("UniFFIRuntime::liftString({value})"),
        Type::Bytes => format!("UniFFIRuntime::liftBytes({value})"),
        Type::Object { name, .. }
        | Type::Record { name, .. }
        | Type::Enum { name, .. }
        | Type::CallbackInterface { name, .. } => {
            format!("{}::uniffiLift({value})", class_name(name))
        }
        Type::Optional { .. }
        | Type::Sequence { .. }
        | Type::Map { .. }
        | Type::Timestamp
        | Type::Duration
        | Type::Custom { .. } => {
            format!(
                "UniFFIRuntime::liftSerialized({value}, {})",
                type_spec(type_)
            )
        }
    })
}

fn type_adapter<'a>(
    type_: &Type,
    type_adapters: &'a PhpTypeAdapters,
) -> Option<&'a PhpTypeAdapter> {
    let name = match type_ {
        Type::Object { name, .. }
        | Type::Record { name, .. }
        | Type::Enum { name, .. }
        | Type::CallbackInterface { name, .. } => name,
        _ => return None,
    };

    type_adapters
        .get(name)
        .or_else(|| type_adapters.get(&class_name(name)))
}

pub(crate) fn class_name(name: &str) -> String {
    avoid_php_reserved(name.to_upper_camel_case())
}

pub(crate) fn function_name(name: &str) -> String {
    avoid_php_reserved(name.to_lower_camel_case())
}

pub(crate) fn type_spec(type_: &Type) -> String {
    match type_ {
        Type::UInt8 => "'u8'".to_string(),
        Type::Int8 => "'i8'".to_string(),
        Type::UInt16 => "'u16'".to_string(),
        Type::Int16 => "'i16'".to_string(),
        Type::UInt32 => "'u32'".to_string(),
        Type::Int32 => "'i32'".to_string(),
        Type::UInt64 => "'u64'".to_string(),
        Type::Int64 => "'i64'".to_string(),
        Type::Float32 => "'f32'".to_string(),
        Type::Float64 => "'f64'".to_string(),
        Type::Boolean => "'bool'".to_string(),
        Type::String => "'string'".to_string(),
        Type::Bytes => "'bytes'".to_string(),
        Type::Timestamp => "'timestamp'".to_string(),
        Type::Duration => "'duration'".to_string(),
        Type::Object { name, imp, .. } => {
            let kind = match imp {
                ObjectImpl::CallbackTrait => "callback-object",
                _ => "object",
            };
            format!("['{kind}', '{}']", class_name(name))
        }
        Type::Record { name, .. } => format!("['record', '{}']", class_name(name)),
        Type::Enum { name, .. } => format!("['enum', '{}']", class_name(name)),
        Type::CallbackInterface { name, .. } => format!("['callback', '{}']", class_name(name)),
        Type::Optional { inner_type } => format!("['optional', {}]", type_spec(inner_type)),
        Type::Sequence { inner_type } => format!("['sequence', {}]", type_spec(inner_type)),
        Type::Map {
            key_type,
            value_type,
        } => {
            format!(
                "['map', {}, {}]",
                type_spec(key_type),
                type_spec(value_type)
            )
        }
        Type::Custom { builtin, .. } => type_spec(builtin),
    }
}

pub(crate) fn variable_name(name: &str) -> String {
    avoid_php_reserved(name.to_lower_camel_case())
}

fn avoid_php_reserved(name: String) -> String {
    if PHP_RESERVED_WORDS
        .iter()
        .any(|word| word.eq_ignore_ascii_case(&name))
    {
        format!("{name}_")
    } else {
        name
    }
}

const PHP_RESERVED_WORDS: &[&str] = &[
    "__halt_compiler",
    "abstract",
    "and",
    "array",
    "as",
    "break",
    "callable",
    "case",
    "catch",
    "class",
    "clone",
    "const",
    "continue",
    "declare",
    "default",
    "die",
    "do",
    "echo",
    "else",
    "elseif",
    "empty",
    "enddeclare",
    "endfor",
    "endforeach",
    "endif",
    "endswitch",
    "endwhile",
    "eval",
    "exit",
    "extends",
    "final",
    "finally",
    "fn",
    "for",
    "foreach",
    "function",
    "global",
    "goto",
    "if",
    "implements",
    "include",
    "include_once",
    "instanceof",
    "insteadof",
    "interface",
    "isset",
    "list",
    "match",
    "namespace",
    "new",
    "or",
    "print",
    "private",
    "protected",
    "public",
    "readonly",
    "require",
    "require_once",
    "return",
    "static",
    "switch",
    "throw",
    "trait",
    "try",
    "unset",
    "use",
    "var",
    "while",
    "xor",
    "yield",
];
