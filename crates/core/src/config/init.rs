//! Configuration initialization for `config init`.

use std::fs;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_error::{NestError, NestResult};
use serde::Serialize;

const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../../../../config.example.toml");

/// JSON response for successful `config init` with `--json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigInitResult {
    /// Absolute path to the created configuration file.
    pub created: PathBuf,
}

/// Creates a default `config.toml` from the embedded template.
pub fn init(ctx: &AppContext, matches: &ArgMatches) -> NestResult<()> {
    let globals = ctx
        .service::<CliGlobals>()
        .map(|globals| globals.clone())
        .unwrap_or_else(|_| default_globals());
    let force = matches.get_flag("force");
    let path = resolve_init_output_path(&globals, matches);

    if path.exists() && !force {
        return Err(NestError::config(format!(
            "configuration file already exists: {}",
            path.display()
        ))
        .with_help("Use --force to overwrite the existing file."));
    }

    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|error| {
            NestError::io(format!(
                "failed to create directory {}: {error}",
                parent.display()
            ))
        })?;
    }

    fs::write(&path, DEFAULT_CONFIG_TEMPLATE).map_err(|error| {
        NestError::io(format!(
            "failed to write configuration file {}: {error}",
            path.display()
        ))
    })?;

    let created = absolute_path(&path);
    let quiet = globals.quiet;
    let json = globals.json;

    if json {
        let payload = serde_json::to_string_pretty(&ConfigInitResult { created }).map_err(
            |error| NestError::data(format!("failed to serialize init result: {error}")),
        )?;
        println!("{payload}");
    } else if !quiet {
        println!("Created configuration: {}", created.display());
    }

    Ok(())
}

/// Resolves the output path for `config init`.
pub fn resolve_init_output_path(globals: &CliGlobals, matches: &ArgMatches) -> PathBuf {
    if let Some(path) = &globals.config_path {
        return path.clone();
    }
    if let Some(output) = matches.get_one::<String>("output") {
        return PathBuf::from(output);
    }
    PathBuf::from("config.toml")
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn default_globals() -> CliGlobals {
    CliGlobals {
        config_path: None,
        log_level: None,
        log_file: None,
        json: false,
        quiet: false,
        verbose: false,
        no_color: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, ArgAction, Command};

    fn init_matches(output: Option<&str>, force: bool) -> ArgMatches {
        let cmd = Command::new("init")
            .arg(
                Arg::new("force")
                    .long("force")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("output")
                    .long("output")
                    .required(false),
            );

        let mut args = vec!["init"];
        if force {
            args.push("--force");
        }
        if let Some(output) = output {
            args.push("--output");
            args.push(output);
        }

        cmd.try_get_matches_from(args).unwrap()
    }

    #[test]
    fn resolve_output_prefers_global_config() {
        let globals = CliGlobals {
            config_path: Some(PathBuf::from("/tmp/from-global.toml")),
            log_level: None,
            log_file: None,
            json: false,
            quiet: false,
            verbose: false,
            no_color: false,
        };
        let matches = init_matches(None, false);
        assert_eq!(
            resolve_init_output_path(&globals, &matches),
            PathBuf::from("/tmp/from-global.toml")
        );
    }

    #[test]
    fn resolve_output_uses_subcommand_flag() {
        let globals = default_globals();
        let matches = init_matches(Some("custom.toml"), false);
        assert_eq!(
            resolve_init_output_path(&globals, &matches),
            PathBuf::from("custom.toml")
        );
    }

    #[test]
    fn resolve_output_defaults_to_config_toml() {
        let globals = default_globals();
        let matches = init_matches(None, false);
        assert_eq!(
            resolve_init_output_path(&globals, &matches),
            PathBuf::from("config.toml")
        );
    }
}
