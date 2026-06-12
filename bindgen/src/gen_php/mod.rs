use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use askama::Template;
use camino::Utf8Path;
use fs_err as fs;
use heck::ToUpperCamelCase;
use serde::{Deserialize, Serialize};
use uniffi_bindgen::interface::{
    rename, AsType, Callable, CallbackInterface, ComponentInterface, Enum as UniFfiEnum,
    FfiCallbackFunction, FfiDefinition, FfiFunction, FfiStruct, FfiType, Field, Record,
};
use uniffi_bindgen::{
    cargo_metadata::CrateConfigSupplier, generate_external_bindings, library_mode,
    BindingGenerator, Component, GenerationSettings,
};

mod object;

use object::{
    build_function, build_object, class_name, function_name, lift_expr, lower_expr, php_type_hint,
    type_spec, validate_type, variable_name, PhpCallable, PhpObject, PhpTypeAdapters,
};

#[derive(Debug, Clone, Default)]
pub struct PhpBindingGenerator {
    cdylib_path: Option<String>,
}

impl PhpBindingGenerator {
    pub fn new(cdylib_path: Option<String>) -> Self {
        Self { cdylib_path }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub cdylib_name: Option<String>,
    pub cdylib_path: Option<String>,
    pub namespace: Option<String>,
    pub file_name: Option<String>,
    pub omit_checksums: bool,
    pub exclude: Vec<String>,
    pub rename: toml::Table,
    pub type_adapters: PhpTypeAdapters,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cdylib_name: None,
            cdylib_path: None,
            namespace: None,
            file_name: None,
            omit_checksums: false,
            exclude: Vec::new(),
            rename: toml::Table::new(),
            type_adapters: PhpTypeAdapters::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpTypeAdapter {
    pub type_hint: String,
    pub lower: String,
    #[serde(default)]
    pub code: String,
}

impl Config {
    fn php_namespace(&self, ci: &ComponentInterface) -> String {
        self.namespace
            .clone()
            .unwrap_or_else(|| ci.namespace().to_upper_camel_case())
    }

    fn output_file_name(&self, ci: &ComponentInterface) -> String {
        self.file_name
            .clone()
            .unwrap_or_else(|| format!("{}.php", ci.namespace()))
    }

    fn library_name(&self) -> String {
        self.cdylib_path
            .clone()
            .or_else(|| self.cdylib_name.clone())
            .unwrap_or_else(|| "uniffi".to_string())
    }
}

impl BindingGenerator for PhpBindingGenerator {
    type Config = Config;

    fn new_config(&self, root_toml: &toml::Value) -> Result<Self::Config> {
        match root_toml.get("bindings").and_then(|b| b.get("php")) {
            Some(v) => Ok(v.clone().try_into()?),
            None => Ok(Config::default()),
        }
    }

    fn update_component_configs(
        &self,
        settings: &GenerationSettings,
        components: &mut Vec<Component<Self::Config>>,
    ) -> Result<()> {
        for component in components.iter_mut() {
            if let Some(path) = &self.cdylib_path {
                component
                    .config
                    .cdylib_path
                    .get_or_insert_with(|| path.clone());
            }
            component.config.cdylib_name.get_or_insert_with(|| {
                settings
                    .cdylib
                    .clone()
                    .unwrap_or_else(|| format!("uniffi_{}", component.ci.namespace()))
            });
        }

        apply_renames(components);
        Ok(())
    }

    fn write_bindings(
        &self,
        settings: &GenerationSettings,
        components: &[Component<Self::Config>],
    ) -> Result<()> {
        fs::create_dir_all(&settings.out_dir)?;

        for Component { ci, config } in components {
            let php = render_php_bindings(config, ci)?;
            fs::write(settings.out_dir.join(config.output_file_name(ci)), php)?;
        }
        Ok(())
    }
}

pub fn generate_php_bindings(
    binding_generator: &PhpBindingGenerator,
    udl_file: &Utf8Path,
    config_file_override: Option<&Utf8Path>,
    out_dir_override: Option<&Utf8Path>,
    library_file: Option<&Utf8Path>,
    crate_name: Option<&str>,
    try_format_code: bool,
) -> Result<()> {
    generate_external_bindings(
        binding_generator,
        udl_file,
        config_file_override,
        out_dir_override,
        library_file,
        crate_name,
        try_format_code,
    )
}

pub fn generate_php_bindings_from_library(
    binding_generator: &PhpBindingGenerator,
    library_file: &Utf8Path,
    config_file_override: Option<&Utf8Path>,
    out_dir: &Utf8Path,
    crate_name: Option<&str>,
    try_format_code: bool,
) -> Result<()> {
    let config_supplier = CrateConfigSupplier::from_cargo_metadata_command(false)?;
    library_mode::generate_bindings(
        library_file,
        crate_name.map(ToOwned::to_owned),
        binding_generator,
        &config_supplier,
        config_file_override,
        out_dir,
        try_format_code,
    )?;
    Ok(())
}

pub fn render_php_bindings(config: &Config, ci: &ComponentInterface) -> Result<String> {
    validate_component(ci)?;

    let functions = ci
        .function_definitions()
        .iter()
        .map(|f| build_function(f.name(), f, &config.type_adapters))
        .collect::<Result<Vec<_>>>()?;
    let objects = ci
        .object_definitions()
        .iter()
        .map(|obj| build_object(obj, &config.type_adapters))
        .collect::<Result<Vec<_>>>()?;

    PhpWrapper {
        namespace: config.php_namespace(ci),
        cdef: render_cdef(ci)?,
        library: php_single_quoted_content(&config.library_name()),
        contract_version: ci.uniffi_contract_version(),
        contract_version_fn: ci.ffi_uniffi_contract_version().name().to_string(),
        rustbuffer_from_bytes_fn: ci.ffi_rustbuffer_from_bytes().name().to_string(),
        rustbuffer_free_fn: ci.ffi_rustbuffer_free().name().to_string(),
        omit_checksums: config.omit_checksums,
        checksums: ci
            .iter_checksums()
            .map(|(fn_name, checksum)| Checksum { fn_name, checksum })
            .collect(),
        type_definitions: render_type_definitions(ci)?,
        callback_initializers: render_callback_initializers(ci),
        functions,
        objects,
        custom_code: render_type_adapter_code(config),
    }
    .render()
    .context("failed to render PHP bindings")
}

fn render_type_adapter_code(config: &Config) -> String {
    config
        .type_adapters
        .values()
        .filter_map(|adapter| {
            let code = adapter.code.trim();
            (!code.is_empty()).then(|| code.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn validate_component(ci: &ComponentInterface) -> Result<()> {
    if ci.has_async_fns() {
        bail!("PHP bindings do not support async UniFFI APIs yet");
    }
    for callable in ci.iter_callables() {
        validate_callable(callable)?;
    }
    Ok(())
}

fn validate_callable(callable: &dyn Callable) -> Result<()> {
    if callable.is_async() {
        bail!("PHP bindings do not support async callables yet");
    }
    for arg in callable.arguments() {
        validate_type(&arg.as_type())?;
    }
    if let Some(return_type) = callable.return_type() {
        validate_type(return_type)?;
    }
    if let Some(throws_type) = callable.throws_type() {
        validate_type(throws_type)?;
    }
    Ok(())
}

fn apply_renames(components: &mut Vec<Component<Config>>) {
    let mut module_renames = HashMap::new();
    for component in components.iter() {
        if !component.config.rename.is_empty() {
            module_renames.insert(
                component.ci.crate_name().to_string(),
                component.config.rename.clone(),
            );
        }
    }

    if !module_renames.is_empty() {
        for component in components {
            rename(&mut component.ci, &module_renames);
        }
    }
}

#[derive(Template)]
#[template(escape = "none", path = "RecordTemplate.php")]
struct PhpRecordDefinition {
    class_name: String,
    constructor_args: String,
    read_args: String,
    write_fields: Vec<PhpRecordWriteField>,
}

#[derive(Debug, Clone)]
struct PhpRecordWriteField {
    property: String,
    spec: String,
}

#[derive(Template)]
#[template(escape = "none", path = "EnumTemplate.php")]
struct PhpEnumDefinition {
    class_name: String,
    variants: Vec<PhpEnumVariant>,
}

#[derive(Debug, Clone)]
struct PhpEnumVariant {
    method_name: String,
    variant_literal: String,
    factory_args: String,
    field_array: String,
    discr: i64,
    read_args: String,
    write_fields: Vec<PhpEnumField>,
}

#[derive(Debug, Clone)]
struct PhpEnumField {
    property: String,
    property_literal: String,
    type_hint: String,
    spec: String,
}

#[derive(Template)]
#[template(escape = "none", path = "CallbackInterfaceTemplate.php")]
struct PhpCallbackInterfaceDefinition {
    class_name: String,
    methods: Vec<PhpCallbackMethod>,
}

#[derive(Debug, Clone)]
struct PhpCallbackMethod {
    name: String,
    args_decl: String,
    return_type_decl: String,
}

#[derive(Template)]
#[template(escape = "none", path = "CallbackVTableTemplate.php")]
struct PhpCallbackVTableDefinition {
    helper_class: String,
    class_name: String,
    vtable_name: String,
    init_fn: String,
    methods: Vec<PhpCallbackVTableMethod>,
}

#[derive(Debug, Clone)]
struct PhpCallbackVTableMethod {
    ffi_field_name: String,
    closure_args: String,
    invoke_line: String,
    write_return: String,
}

fn render_type_definitions(ci: &ComponentInterface) -> Result<String> {
    let mut out = String::new();

    for record in ci.record_definitions() {
        out.push_str(&render_record_definition(record)?);
        out.push('\n');
    }

    for enum_ in ci.enum_definitions() {
        out.push_str(&render_enum_definition(ci, enum_)?);
        out.push('\n');
    }

    for callback in ci.callback_interface_definitions() {
        out.push_str(&render_callback_interface_definition(callback)?);
        out.push('\n');
    }

    for object in ci
        .object_definitions()
        .iter()
        .filter(|o| o.has_callback_interface())
    {
        out.push_str(&render_callback_vtable_definition(
            &class_name(object.name()),
            &callback_helper_class_name(object.name()),
            object.ffi_init_callback().name(),
            &object.vtable_methods(),
        )?);
        out.push('\n');
    }

    for callback in ci.callback_interface_definitions() {
        out.push_str(&render_callback_vtable_definition(
            &class_name(callback.name()),
            &callback_helper_class_name(callback.name()),
            callback.ffi_init_callback().name(),
            &callback.vtable_methods(),
        )?);
        out.push('\n');
    }

    Ok(out)
}

fn render_callback_initializers(ci: &ComponentInterface) -> Vec<String> {
    ci.object_definitions()
        .iter()
        .filter(|object| object.has_callback_interface())
        .map(|object| format!("{}::init();", callback_helper_class_name(object.name())))
        .chain(
            ci.callback_interface_definitions().iter().map(|callback| {
                format!("{}::init();", callback_helper_class_name(callback.name()))
            }),
        )
        .collect()
}

fn render_record_definition(record: &Record) -> Result<String> {
    let class = class_name(record.name());
    let constructor_args = record
        .fields()
        .iter()
        .enumerate()
        .map(|(index, field)| {
            Ok(format!(
                "public {} ${}",
                php_type_hint(&field.as_type())?,
                field_php_name(field, index)
            ))
        })
        .collect::<Result<Vec<_>>>()?
        .join(", ");

    let read_args = record
        .fields()
        .iter()
        .map(|field| {
            Ok(format!(
                "UniFFIRuntime::readValue($reader, {})",
                type_spec(&field.as_type())
            ))
        })
        .collect::<Result<Vec<_>>>()?
        .join(", ");

    let write_fields = record
        .fields()
        .iter()
        .enumerate()
        .map(|(index, field)| PhpRecordWriteField {
            property: field_php_name(field, index),
            spec: type_spec(&field.as_type()),
        })
        .collect();

    PhpRecordDefinition {
        class_name: class,
        constructor_args,
        read_args,
        write_fields,
    }
    .render()
    .context("failed to render PHP record definition")
}

fn render_enum_definition(ci: &ComponentInterface, enum_: &UniFfiEnum) -> Result<String> {
    let class = class_name(enum_.name());
    let is_error = ci.is_name_used_as_error(enum_.name());
    let is_flat_error = is_error && enum_.is_flat();
    let mut variants = Vec::new();

    for (variant_index, variant) in enum_.variants().iter().enumerate() {
        let method = function_name(variant.name());
        let variant_literal = php_single_quoted_literal(variant.name());
        let discr = i64::try_from(variant_index + 1).context("enum discriminant out of range")?;
        let fields = if is_flat_error {
            vec![PhpEnumField {
                property: "message".to_string(),
                property_literal: "'message'".to_string(),
                type_hint: "string".to_string(),
                spec: "'string'".to_string(),
            }]
        } else {
            variant
                .fields()
                .iter()
                .enumerate()
                .map(|(field_index, field)| {
                    let property = field_php_name(field, field_index);
                    Ok(PhpEnumField {
                        property_literal: php_single_quoted_literal(&property),
                        property,
                        type_hint: php_type_hint(&field.as_type())?,
                        spec: type_spec(&field.as_type()),
                    })
                })
                .collect::<Result<Vec<_>>>()?
        };
        let write_fields = if is_flat_error {
            Vec::new()
        } else {
            fields.clone()
        };

        let factory_args = fields
            .iter()
            .map(|field| format!("{} ${}", field.type_hint, field.property))
            .collect::<Vec<_>>()
            .join(", ");
        let field_array = fields
            .iter()
            .map(|field| format!("{} => ${}", field.property_literal, field.property))
            .collect::<Vec<_>>()
            .join(", ");

        let read_args = fields
            .iter()
            .map(|field| format!("UniFFIRuntime::readValue($reader, {})", field.spec))
            .collect::<Vec<_>>()
            .join(", ");
        variants.push(PhpEnumVariant {
            method_name: method,
            variant_literal,
            factory_args,
            field_array,
            discr,
            read_args,
            write_fields,
        });
    }

    PhpEnumDefinition {
        class_name: class,
        variants,
    }
    .render()
    .context("failed to render PHP enum definition")
}

fn render_callback_interface_definition(callback: &CallbackInterface) -> Result<String> {
    let class = class_name(callback.name());
    let type_adapters = PhpTypeAdapters::new();
    let methods = callback
        .methods()
        .into_iter()
        .map(|method| {
            let callable = build_function(method.name(), method, &type_adapters)?;
            Ok(PhpCallbackMethod {
                name: callable.name,
                args_decl: callable.args_decl,
                return_type_decl: callable.return_type_decl,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    PhpCallbackInterfaceDefinition {
        class_name: class,
        methods,
    }
    .render()
    .context("failed to render PHP callback interface definition")
}

fn render_callback_vtable_definition(
    class: &str,
    helper_class: &str,
    init_fn: &str,
    methods: &[(FfiCallbackFunction, uniffi_bindgen::interface::Method)],
) -> Result<String> {
    let methods = methods
        .iter()
        .map(|(ffi_callback, method)| render_callback_method(class, ffi_callback, method))
        .collect::<Result<Vec<_>>>()?;

    PhpCallbackVTableDefinition {
        helper_class: helper_class.to_string(),
        class_name: class.to_string(),
        vtable_name: callback_vtable_name(class),
        init_fn: init_fn.to_string(),
        methods,
    }
    .render()
    .context("failed to render PHP callback vtable definition")
}

fn render_callback_method(
    _class: &str,
    ffi_callback: &FfiCallbackFunction,
    method: &uniffi_bindgen::interface::Method,
) -> Result<PhpCallbackVTableMethod> {
    let closure_args = ffi_callback
        .arguments()
        .into_iter()
        .map(|arg| format!("${}", variable_name(arg.name())))
        .chain(
            ffi_callback
                .has_rust_call_status_arg()
                .then(|| "$uniffiCallStatus".to_string()),
        )
        .collect::<Vec<_>>()
        .join(", ");

    let call_args = method
        .arguments()
        .into_iter()
        .map(|arg| lift_expr(&format!("${}", variable_name(arg.name())), &arg.as_type()))
        .collect::<Result<Vec<_>>>()?
        .join(", ");

    let method_name = function_name(method.name());
    let invoke = if call_args.is_empty() {
        format!("$obj->{method_name}()")
    } else {
        format!("$obj->{method_name}({call_args})")
    };
    let (invoke_line, write_return) = match method.return_type() {
        Some(return_type) => {
            let lowered = lower_expr("$result", return_type)?;
            (
                format!("                $result = {invoke};\n"),
                format!("            $uniffiOutReturn[0] = {lowered};\n"),
            )
        }
        None => (format!("                {invoke};\n"), String::new()),
    };

    Ok(PhpCallbackVTableMethod {
        ffi_field_name: method.name().to_string(),
        closure_args,
        invoke_line,
        write_return,
    })
}

fn field_php_name(field: &Field, index: usize) -> String {
    if field.name().is_empty() {
        format!("field{index}")
    } else {
        variable_name(field.name())
    }
}

fn callback_helper_class_name(name: &str) -> String {
    format!("UniFFICallbacks{}", class_name(name))
}

fn callback_vtable_name(class: &str) -> String {
    format!("VTableCallbackInterface{class}")
}

fn php_single_quoted_literal(value: &str) -> String {
    format!("'{}'", php_single_quoted_content(value))
}

fn render_cdef(ci: &ComponentInterface) -> Result<String> {
    let mut out = String::from(
        r#"typedef signed char int8_t;
typedef unsigned char uint8_t;
typedef short int16_t;
typedef unsigned short uint16_t;
typedef int int32_t;
typedef unsigned int uint32_t;
typedef long long int64_t;
typedef unsigned long long uint64_t;

typedef struct RustBuffer {
    uint64_t capacity;
    uint64_t len;
    uint8_t *data;
} RustBuffer;

typedef struct ForeignBytes {
    int32_t len;
    const uint8_t *data;
} ForeignBytes;

typedef struct RustCallStatus {
    int8_t code;
    RustBuffer errorBuf;
} RustCallStatus;

"#,
    );

    for definition in ci.ffi_definitions() {
        match definition {
            FfiDefinition::Function(function) => {
                if function.is_async() {
                    bail!("PHP bindings do not support async FFI functions yet");
                }
                out.push_str(&render_c_function(&function)?);
                out.push('\n');
            }
            FfiDefinition::CallbackFunction(callback) => {
                out.push_str(&render_c_callback_function(&callback)?);
                out.push('\n');
            }
            FfiDefinition::Struct(struct_) => {
                out.push_str(&render_c_struct(&struct_)?);
                out.push('\n');
            }
        }
    }
    Ok(out)
}

fn render_c_function(function: &FfiFunction) -> Result<String> {
    let return_type = match function.return_type() {
        Some(type_) => c_type(type_)?,
        None => "void".to_string(),
    };
    let mut args = function
        .arguments()
        .into_iter()
        .map(|arg| Ok(format!("{} {}", c_type(&arg.type_())?, arg.name())))
        .collect::<Result<Vec<_>>>()?;
    if function.has_rust_call_status_arg() {
        args.push("RustCallStatus *out_status".to_string());
    }
    if args.is_empty() {
        args.push("void".to_string());
    }

    Ok(format!(
        "{return_type} {}({});",
        function.name(),
        args.join(", ")
    ))
}

fn render_c_callback_function(function: &FfiCallbackFunction) -> Result<String> {
    let return_type = match function.return_type() {
        Some(type_) => c_type(type_)?,
        None => "void".to_string(),
    };
    let mut args = function
        .arguments()
        .into_iter()
        .map(|arg| Ok(format!("{} {}", c_type(&arg.type_())?, arg.name())))
        .collect::<Result<Vec<_>>>()?;
    if function.has_rust_call_status_arg() {
        args.push("RustCallStatus *out_status".to_string());
    }
    if args.is_empty() {
        args.push("void".to_string());
    }

    Ok(format!(
        "typedef {return_type} (*{})({});",
        function.name(),
        args.join(", ")
    ))
}

fn render_c_struct(struct_: &FfiStruct) -> Result<String> {
    let mut out = format!("typedef struct {} {{\n", struct_.name());
    for field in struct_.fields() {
        out.push_str(&format!(
            "    {} {};\n",
            c_type(&field.type_())?,
            field.name()
        ));
    }
    out.push_str(&format!("}} {};\n", struct_.name()));
    Ok(out)
}

fn c_type(type_: &FfiType) -> Result<String> {
    Ok(match type_ {
        FfiType::UInt8 => "uint8_t".to_string(),
        FfiType::Int8 => "int8_t".to_string(),
        FfiType::UInt16 => "uint16_t".to_string(),
        FfiType::Int16 => "int16_t".to_string(),
        FfiType::UInt32 => "uint32_t".to_string(),
        FfiType::Int32 => "int32_t".to_string(),
        FfiType::UInt64 | FfiType::Handle => "uint64_t".to_string(),
        FfiType::Int64 => "int64_t".to_string(),
        FfiType::Float32 => "float".to_string(),
        FfiType::Float64 => "double".to_string(),
        FfiType::RustBuffer(_) => "RustBuffer".to_string(),
        FfiType::RustCallStatus => "RustCallStatus".to_string(),
        FfiType::ForeignBytes => "ForeignBytes".to_string(),
        FfiType::Reference(inner) | FfiType::MutReference(inner) => {
            format!("{} *", c_type(inner)?)
        }
        FfiType::VoidPointer => "void *".to_string(),
        FfiType::Callback(name) | FfiType::Struct(name) => name.to_string(),
    })
}

fn php_single_quoted_content(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

#[derive(Debug, Clone)]
pub(crate) struct Checksum {
    pub(crate) fn_name: String,
    pub(crate) checksum: u16,
}

#[derive(Template)]
#[template(escape = "none", path = "wrapper.php")]
struct PhpWrapper {
    namespace: String,
    cdef: String,
    library: String,
    contract_version: u32,
    contract_version_fn: String,
    rustbuffer_from_bytes_fn: String,
    rustbuffer_free_fn: String,
    omit_checksums: bool,
    checksums: Vec<Checksum>,
    type_definitions: String,
    callback_initializers: Vec<String>,
    functions: Vec<PhpCallable>,
    objects: Vec<PhpObject>,
    custom_code: String,
}
