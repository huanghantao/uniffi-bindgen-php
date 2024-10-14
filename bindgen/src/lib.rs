/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

pub mod gen_php;

use camino::Utf8PathBuf;
use clap::Parser;
use gen_php::BindingGeneratorPHP;

#[derive(Parser)]
#[clap(name = "uniffi-bindgen")]
#[clap(version = clap::crate_version!())]
#[clap(propagate_version = true)]
struct Cli {
    /// Directory in which to write generated files. Default is same folder as .udl file.
    #[clap(long, short)]
    out_dir: Option<Utf8PathBuf>,

    /// Do not try to format the generated bindings.
    #[clap(long, short)]
    no_format: bool,

    /// Path to optional uniffi config file. This config will be merged on top of default
    /// `uniffi.toml` config in crate root. The merge recursively upserts TOML keys into
    /// the default config.
    #[clap(long, short)]
    config: Option<Utf8PathBuf>,

    /// Extract proc-macro metadata from a native lib (cdylib or staticlib) for this crate.
    #[clap(long, short)]
    lib_file: Option<Utf8PathBuf>,

    /// Pass in a cdylib path rather than a UDL file
    #[clap(long = "library")]
    library_mode: bool,

    /// When `--library` is passed, only generate bindings for one crate.
    /// When `--library` is not passed, use this as the crate name instead of attempting to
    /// locate and parse Cargo.toml.
    #[clap(long = "crate")]
    crate_name: Option<String>,

    /// Path to the UDL file, or cdylib if `library-mode` is specified
    source: Utf8PathBuf,
}

pub fn main() -> anyhow::Result<()> {
    let Cli {
        out_dir,
        no_format,
        config,
        lib_file,
        library_mode,
        crate_name,
        source,
    } = Cli::parse();

    let binding_gen = BindingGeneratorPHP {
        try_format_code: !no_format,
    };
    if library_mode {
        if lib_file.is_some() {
            panic!("--lib-file is not compatible with --library.")
        }
        let out_dir = out_dir.expect("--out-dir is required when using --library");
        let library_path = source;

        uniffi_bindgen::library_mode::generate_bindings(
            &library_path,
            crate_name,
            &binding_gen,
            config.as_deref(),
            &out_dir,
            !no_format,
        )?;
    } else {
        let udl_file = source;
        uniffi_bindgen::generate_bindings(
            &udl_file,
            config.as_deref(),
            binding_gen,
            out_dir.as_deref(),
            lib_file.as_deref(),
            crate_name.as_deref(),
            !no_format,
        )?;
    }

    Ok(())
}
