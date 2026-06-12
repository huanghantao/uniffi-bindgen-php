use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use uniffi_bindgen_php::{
    generate_php_bindings, generate_php_bindings_from_library, PhpBindingGenerator,
};

#[derive(Debug)]
struct Cli {
    source: Option<Utf8PathBuf>,
    out_dir: Utf8PathBuf,
    config: Option<Utf8PathBuf>,
    library: Option<Utf8PathBuf>,
    crate_name: Option<String>,
    format: bool,
}

fn main() -> Result<()> {
    let cli = parse_args()?;
    let generator = PhpBindingGenerator::new(cli.library.as_ref().map(ToString::to_string));
    match (cli.source.as_deref(), cli.library.as_deref()) {
        (Some(source), library) => generate_php_bindings(
            &generator,
            source,
            cli.config.as_deref(),
            Some(cli.out_dir.as_path()),
            library,
            cli.crate_name.as_deref(),
            cli.format,
        ),
        (None, Some(library)) => generate_php_bindings_from_library(
            &generator,
            library,
            cli.config.as_deref(),
            cli.out_dir.as_path(),
            cli.crate_name.as_deref(),
            cli.format,
        ),
        (None, None) => bail!("missing input: pass a UDL file or --library <cdylib>"),
    }
}

fn parse_args() -> Result<Cli> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "generate") {
        args.remove(0);
    }

    let mut source = None;
    let mut out_dir = Utf8PathBuf::from(".");
    let mut config = None;
    let mut library = None;
    let mut crate_name = None;
    let mut format = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "-o" | "--out-dir" => {
                i += 1;
                out_dir = next_path(&args, i, "--out-dir")?;
            }
            "-c" | "--config" => {
                i += 1;
                config = Some(next_path(&args, i, "--config")?);
            }
            "-l" | "--library" | "--library-file" => {
                i += 1;
                library = Some(next_path(&args, i, "--library")?);
            }
            "--crate" | "--crate-name" => {
                i += 1;
                crate_name = Some(next_value(&args, i, "--crate")?.to_string());
            }
            "--format" => {
                format = true;
            }
            "--no-format" => {
                format = false;
            }
            arg if arg.starts_with('-') => bail!("unknown option: {arg}"),
            arg => {
                if source.replace(Utf8PathBuf::from(arg)).is_some() {
                    bail!("multiple input UDL files supplied");
                }
            }
        }
        i += 1;
    }

    Ok(Cli {
        source,
        out_dir,
        config,
        library,
        crate_name,
        format,
    })
}

fn next_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str> {
    args.get(index)
        .map(String::as_str)
        .with_context(|| format!("{option} requires a value"))
}

fn next_path(args: &[String], index: usize, option: &str) -> Result<Utf8PathBuf> {
    Ok(Utf8PathBuf::from(next_value(args, index, option)?))
}

fn print_usage() {
    eprintln!(
        "Usage: uniffi-bindgen-php generate [<udl-file>] [--library LIB] [--out-dir DIR] [--crate NAME] [--config FILE]"
    );
}
