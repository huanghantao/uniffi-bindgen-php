/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::{Context, Result};
use askama::Template;
use fs_err::{self as fs};
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToUpperCamelCase};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};
use uniffi_bindgen::{
    backend::{CodeType, TemplateExpression},
    interface::{FfiType, Object},
    Component, ComponentInterface, GenerationSettings,
};
use uniffi_meta::Type;

static KEYWORDS: Lazy<HashSet<String>> = Lazy::new(|| {
    [
        // Keywords used in declarations:
        "associatedtype",
        "class",
        "deinit",
        "enum",
        "extension",
        "fileprivate",
        "func",
        "import",
        "init",
        "inout",
        "internal",
        "let",
        "open",
        "operator",
        "private",
        "precedencegroup",
        "protocol",
        "public",
        "rethrows",
        "static",
        "struct",
        "subscript",
        "typealias",
        "var",
        // Keywords used in statements:
        "break",
        "case",
        "catch",
        "continue",
        "default",
        "defer",
        "do",
        "else",
        "fallthrough",
        "for",
        "guard",
        "if",
        "in",
        "repeat",
        "return",
        "throw",
        "switch",
        "where",
        "while",
        // Keywords used in expressions and types:
        "Any",
        "as",
        "await",
        "catch",
        "false",
        "is",
        "nil",
        "rethrows",
        "self",
        "Self",
        "super",
        "throw",
        "throws",
        "true",
        "try",
    ]
    .iter()
    .map(ToString::to_string)
    .collect::<HashSet<_>>()
});

pub fn quote_general_keyword(nm: String) -> String {
    if KEYWORDS.contains(&nm) {
        format!("`{nm}`")
    } else {
        nm
    }
}

static ARG_KEYWORDS: Lazy<HashSet<String>> = Lazy::new(|| {
    ["inout", "var", "let"]
        .iter()
        .map(ToString::to_string)
        .collect::<HashSet<_>>()
});

pub fn quote_arg_keyword(nm: String) -> String {
    if ARG_KEYWORDS.contains(&nm) {
        format!("`{nm}`")
    } else {
        nm
    }
}

pub struct Bindings {
    library: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub(super) module_name: Option<String>,
    pub(super) cdylib_name: Option<String>,
    #[serde(default)]
    custom_types: HashMap<String, CustomTypeConfig>,
    #[serde(default)]
    external_packages: HashMap<String, String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CustomTypeConfig {
    imports: Option<Vec<String>>,
    type_name: Option<String>,
    into_custom: TemplateExpression,
    from_custom: TemplateExpression,
}

impl Config {
    pub fn module_name(&self) -> String {
        self.module_name
            .as_ref()
            .expect("module name should have been set in update_component_configs")
            .clone()
    }
}

pub fn generate_bindings(config: &Config, ci: &ComponentInterface) -> Result<Bindings> {
    let library = PhpWrapper::new(config.clone(), ci)
        .render()
        .context("failed to render PHP library")?;

    Ok(Bindings { library })
}

pub struct BindingGeneratorPHP {
    pub try_format_code: bool,
}

impl uniffi_bindgen::BindingGenerator for BindingGeneratorPHP {
    type Config = Config;

    fn new_config(&self, root_toml: &toml::Value) -> anyhow::Result<Self::Config> {
        Ok(match root_toml.get("bindings").and_then(|b| b.get("php")) {
            Some(v) => v.clone().try_into()?,
            None => Default::default(),
        })
    }

    fn update_component_configs(
        &self,
        settings: &GenerationSettings,
        components: &mut Vec<Component<Self::Config>>,
    ) -> anyhow::Result<()> {
        for c in &mut *components {
            c.config
                .module_name
                .get_or_insert_with(|| c.ci.namespace().into());
        }
        Ok(())
    }

    fn write_bindings(
        &self,
        settings: &uniffi_bindgen::GenerationSettings,
        components: &[uniffi_bindgen::Component<Self::Config>],
    ) -> anyhow::Result<()> {
        for Component { ci, config, .. } in components {
            let Bindings { library } = generate_bindings(config, ci)?;

            let source_file = settings
                .out_dir
                .join(format!("{}.php", config.module_name()));
            fs::write(&source_file, library)?;
        }

        Ok(())
    }
}

#[derive(Template)]
#[template(escape = "none", path = "Types.php")]
pub struct TypeRenderer<'a> {
    php_config: &'a Config,
    ci: &'a ComponentInterface,
    include_once_names: RefCell<HashSet<String>>,
}

impl<'a> TypeRenderer<'a> {
    fn new(php_config: &'a Config, ci: &'a ComponentInterface) -> Self {
        Self {
            php_config,
            ci,
            include_once_names: RefCell::new(HashSet::new()),
        }
    }
}

#[derive(Template)]
#[template(escape = "none", path = "wrapper.php")]
pub struct PhpWrapper<'a> {
    ci: &'a ComponentInterface,
    config: Config,
    type_helper_code: String,
}

impl<'a> PhpWrapper<'a> {
    pub fn new(config: Config, ci: &'a ComponentInterface) -> Self {
        let type_renderer = TypeRenderer::new(&config, ci);
        let type_helper_code = type_renderer.render().expect("type rendering");
        Self {
            ci,
            config,
            type_helper_code,
        }
    }
}
#[derive(Clone)]
pub struct PHPCodeOracle;

impl PHPCodeOracle {
    fn create_code_type(&self, type_: Type) -> Box<dyn CodeType> {
        match type_ {
            Type::UInt8 => todo!(),
            Type::Int8 => todo!(),
            Type::UInt16 => todo!(),
            Type::Int16 => todo!(),
            Type::UInt32 => todo!(),
            Type::Int32 => todo!(),
            Type::UInt64 => todo!(),
            Type::Int64 => todo!(),
            Type::Float32 => todo!(),
            Type::Float64 => todo!(),
            Type::Boolean => todo!(),
            Type::String => todo!(),
            Type::Bytes => todo!(),

            Type::Timestamp => todo!(),
            Type::Duration => todo!(),

            Type::Enum { name, .. } => todo!(),
            Type::Object { name, imp, .. } => todo!(),
            Type::Record { name, .. } => todo!(),
            Type::CallbackInterface { name, .. } => todo!(),
            Type::Optional { inner_type } => todo!(),
            Type::Sequence { inner_type } => todo!(),
            Type::Map {
                key_type,
                value_type,
            } => todo!(),
            Type::External { name, .. } => todo!(),
            Type::Custom { name, .. } => todo!(),
        }
    }

    fn find(&self, type_: &Type) -> Box<dyn CodeType> {
        self.create_code_type(type_.clone())
    }

    fn class_name(&self, nm: &str) -> String {
        nm.to_string().to_upper_camel_case()
    }

    fn fn_name(&self, nm: &str) -> String {
        nm.to_string().to_lower_camel_case()
    }

    fn var_name(&self, nm: &str) -> String {
        nm.to_string().to_lower_camel_case()
    }

    fn enum_variant_name(&self, nm: &str) -> String {
        nm.to_string().to_lower_camel_case()
    }

    fn ffi_callback_name(&self, nm: &str) -> String {
        format!("Uniffi{}", nm.to_upper_camel_case())
    }

    fn ffi_struct_name(&self, nm: &str) -> String {
        format!("Uniffi{}", nm.to_upper_camel_case())
    }

    fn if_guard_name(&self, nm: &str) -> String {
        format!("UNIFFI_FFIDEF_{}", nm.to_shouty_snake_case())
    }

    fn ffi_type_label(&self, ffi_type: &FfiType) -> String {
        match ffi_type {
            FfiType::Int8 => "Int8".into(),
            FfiType::UInt8 => "UInt8".into(),
            FfiType::Int16 => "Int16".into(),
            FfiType::UInt16 => "UInt16".into(),
            FfiType::Int32 => "Int32".into(),
            FfiType::UInt32 => "UInt32".into(),
            FfiType::Int64 => "Int64".into(),
            FfiType::UInt64 => "UInt64".into(),
            FfiType::Float32 => "Float".into(),
            FfiType::Float64 => "Double".into(),
            FfiType::Handle => "UInt64".into(),
            FfiType::RustArcPtr(_) => "UnsafeMutableRawPointer".into(),
            FfiType::RustBuffer(_) => "RustBuffer".into(),
            FfiType::RustCallStatus => "RustCallStatus".into(),
            FfiType::ForeignBytes => "ForeignBytes".into(),
            FfiType::Callback(name) => format!("@escaping {}", self.ffi_callback_name(name)),
            FfiType::Struct(name) => self.ffi_struct_name(name),
            FfiType::Reference(inner) => {
                format!("UnsafeMutablePointer<{}>", self.ffi_type_label(inner))
            }
            FfiType::VoidPointer => "UnsafeMutableRawPointer".into(),
        }
    }

    fn ffi_default_value(&self, return_type: Option<&FfiType>) -> String {
        match return_type {
            Some(t) => match t {
                FfiType::UInt8
                | FfiType::Int8
                | FfiType::UInt16
                | FfiType::Int16
                | FfiType::UInt32
                | FfiType::Int32
                | FfiType::UInt64
                | FfiType::Int64 => "0".to_owned(),
                FfiType::Float32 | FfiType::Float64 => "0.0".to_owned(),
                FfiType::RustArcPtr(_) => "nil".to_owned(),
                FfiType::RustBuffer(_) => "RustBuffer.empty()".to_owned(),
                _ => unimplemented!("FFI return type: {t:?}"),
            },
            None => "0".to_owned(),
        }
    }

    fn object_names(&self, obj: &Object) -> (String, String) {
        let class_name = self.class_name(obj.name());
        if obj.has_callback_interface() {
            let impl_name = format!("{class_name}Impl");
            (class_name, impl_name)
        } else {
            (format!("{class_name}Protocol"), class_name)
        }
    }
}

pub mod filters {
    use uniffi_bindgen::{backend::Literal, interface::Enum};
    use uniffi_meta::{AsType, LiteralMetadata};

    use super::*;

    fn oracle() -> &'static PHPCodeOracle {
        &PHPCodeOracle
    }

    pub fn type_name(as_type: &impl AsType) -> Result<String, askama::Error> {
        Ok(oracle().find(&as_type.as_type()).type_label())
    }

    pub fn return_type_name(as_type: Option<&impl AsType>) -> Result<String, askama::Error> {
        Ok(match as_type {
            Some(as_type) => oracle().find(&as_type.as_type()).type_label(),
            None => "()".to_owned(),
        })
    }

    pub fn canonical_name(as_type: &impl AsType) -> Result<String, askama::Error> {
        Ok(oracle().find(&as_type.as_type()).canonical_name())
    }

    pub fn ffi_converter_name(as_type: &impl AsType) -> Result<String, askama::Error> {
        Ok(oracle().find(&as_type.as_type()).ffi_converter_name())
    }

    pub fn ffi_error_converter_name(as_type: &impl AsType) -> Result<String, askama::Error> {
        let mut name = oracle().find(&as_type.as_type()).ffi_converter_name();
        if matches!(&as_type.as_type(), Type::Object { .. }) {
            name.push_str("__as_error")
        }
        Ok(name)
    }

    pub fn lower_fn(as_type: &impl AsType) -> Result<String, askama::Error> {
        Ok(oracle().find(&as_type.as_type()).lower())
    }

    pub fn write_fn(as_type: &impl AsType) -> Result<String, askama::Error> {
        Ok(oracle().find(&as_type.as_type()).write())
    }

    pub fn lift_fn(as_type: &impl AsType) -> Result<String, askama::Error> {
        Ok(oracle().find(&as_type.as_type()).lift())
    }

    pub fn read_fn(as_type: &impl AsType) -> Result<String, askama::Error> {
        Ok(oracle().find(&as_type.as_type()).read())
    }

    pub fn literal_php(literal: &Literal, as_type: &impl AsType) -> Result<String, askama::Error> {
        Ok(oracle().find(&as_type.as_type()).literal(literal))
    }

    pub fn variant_discr_literal(e: &Enum, index: &usize) -> Result<String, askama::Error> {
        let literal = e.variant_discr(*index).expect("invalid index");
        match literal {
            LiteralMetadata::UInt(v, _, _) => Ok(v.to_string()),
            LiteralMetadata::Int(v, _, _) => Ok(v.to_string()),
            _ => unreachable!("expected an UInt!"),
        }
    }

    pub fn ffi_type_name(ffi_type: &FfiType) -> Result<String, askama::Error> {
        Ok(oracle().ffi_type_label(ffi_type))
    }

    pub fn ffi_default_value(return_type: Option<FfiType>) -> Result<String, askama::Error> {
        Ok(oracle().ffi_default_value(return_type.as_ref()))
    }

    pub fn header_ffi_type_name(ffi_type: &FfiType) -> Result<String, askama::Error> {
        Ok(match ffi_type {
            FfiType::Int8 => "int8_t".into(),
            FfiType::UInt8 => "uint8_t".into(),
            FfiType::Int16 => "int16_t".into(),
            FfiType::UInt16 => "uint16_t".into(),
            FfiType::Int32 => "int32_t".into(),
            FfiType::UInt32 => "uint32_t".into(),
            FfiType::Int64 => "int64_t".into(),
            FfiType::UInt64 => "uint64_t".into(),
            FfiType::Float32 => "float".into(),
            FfiType::Float64 => "double".into(),
            FfiType::Handle => "uint64_t".into(),
            FfiType::RustArcPtr(_) => "void*_Nonnull".into(),
            FfiType::RustBuffer(_) => "RustBuffer".into(),
            FfiType::RustCallStatus => "RustCallStatus".into(),
            FfiType::ForeignBytes => "ForeignBytes".into(),
            FfiType::Callback(name) => {
                format!("{} _Nonnull", PHPCodeOracle.ffi_callback_name(name))
            }
            FfiType::Struct(name) => PHPCodeOracle.ffi_struct_name(name),
            FfiType::Reference(inner) => format!("{}* _Nonnull", header_ffi_type_name(inner)?),
            FfiType::VoidPointer => "void* _Nonnull".into(),
        })
    }

    pub fn class_name(nm: &str) -> Result<String, askama::Error> {
        Ok(oracle().class_name(nm))
    }

    pub fn fn_name(nm: &str) -> Result<String, askama::Error> {
        Ok(quote_general_keyword(oracle().fn_name(nm)))
    }

    pub fn var_name(nm: &str) -> Result<String, askama::Error> {
        Ok(quote_general_keyword(oracle().var_name(nm)))
    }

    pub fn arg_name(nm: &str) -> Result<String, askama::Error> {
        Ok(quote_arg_keyword(oracle().var_name(nm)))
    }

    pub fn enum_variant_php_quoted(nm: &str) -> Result<String, askama::Error> {
        Ok(quote_general_keyword(oracle().enum_variant_name(nm)))
    }

    pub fn error_variant_php_quoted(nm: &str) -> Result<String, askama::Error> {
        Ok(quote_general_keyword(oracle().class_name(nm)))
    }

    pub fn ffi_callback_name(nm: &str) -> Result<String, askama::Error> {
        Ok(oracle().ffi_callback_name(nm))
    }

    pub fn ffi_struct_name(nm: &str) -> Result<String, askama::Error> {
        Ok(oracle().ffi_struct_name(nm))
    }

    pub fn if_guard_name(nm: &str) -> Result<String, askama::Error> {
        Ok(oracle().if_guard_name(nm))
    }

    pub fn docstring(docstring: &str, spaces: &i32) -> Result<String, askama::Error> {
        let middle = textwrap::indent(&textwrap::dedent(docstring), " * ");
        let wrapped = format!("/**\n{middle}\n */");

        let spaces = usize::try_from(*spaces).unwrap_or_default();
        Ok(textwrap::indent(&wrapped, &" ".repeat(spaces)))
    }

    pub fn object_names(obj: &Object) -> Result<(String, String), askama::Error> {
        Ok(PHPCodeOracle.object_names(obj))
    }
}
