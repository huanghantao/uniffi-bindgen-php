/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::{Context, Result};
use askama::Template;
use fs_err::{self as fs};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};
use uniffi_bindgen::{
    backend::TemplateExpression, Component, ComponentInterface, GenerationSettings,
};

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

/// Renders PHP helper code for all types
///
/// This template is a bit different than others in that it stores internal state from the render
/// process.  Make sure to only call `render()` once.
#[derive(Template)]
#[template(escape = "none", path = "Types.php")]
pub struct TypeRenderer<'a> {
    php_config: &'a Config,
    ci: &'a ComponentInterface,
    // Track included modules for the `include_once()` macro
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
