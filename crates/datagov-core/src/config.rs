//! config.rs — Configuration resolution chain (PRD §28).
//!
//! 1. Defines `Config`, the Bolt 1 configuration surface: `output`
//!    (`OutputFormat`) and `threads`. Deliberately small — the precedence
//!    chain is what this module proves out, not the field surface.
//! 2. Defines `OutputFormat` (`json` | `table` | `csv`, the last added in
//!    Bolt 3 for `datagov query`; kept in lock-step with the clap-facing
//!    `datagov_cli::cli::OutputFormat` used for actual command dispatch).
//! 3. Implements `load`, the precedence chain from PRD §28: CLI flags are
//!    applied by the caller on top of the returned `Config`; this loader
//!    resolves, first hit wins per field, in the order: environment
//!    (`DATAGOV_*`) → `./datagov.yaml` → `./.datagov/config.yaml` →
//!    `$XDG_CONFIG_HOME/datagov/config.yaml` →
//!    `~/.config/datagov/config.yaml` → built-in defaults.
//! 4. Takes the start directory and the environment map as explicit
//!    parameters so the chain is unit-testable without touching the real
//!    filesystem or process environment.
//! 5. Never reads secrets from these files — `Config` carries no
//!    credential fields.
//!
//! Environment variables read (when passed in the `env` map by a caller):
//! `DATAGOV_OUTPUT`, `DATAGOV_THREADS`, `HOME`, `XDG_CONFIG_HOME`.

use crate::error::DatagovError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// The rendering format for command output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Table,
    /// Added in Bolt 3 for `datagov query`; every other command rejects
    /// it explicitly (`DatagovError::InvalidArgs`) rather than silently
    /// falling back to table rendering.
    Csv,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            "csv" => Ok(OutputFormat::Csv),
            other => Err(format!("unknown output format: {other}")),
        }
    }
}

/// The Bolt 1 configuration surface. Fields are `Option` so the merge
/// step can distinguish "unset" from "set to a default-looking value".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub output: Option<OutputFormat>,
    pub threads: Option<usize>,
}

impl Config {
    /// Overwrite only the fields `other` has set, leaving the rest of
    /// `self` untouched. Used to layer configuration sources from lowest
    /// to highest priority.
    fn merge_from(&mut self, other: Config) {
        if let Some(output) = other.output {
            self.output = Some(output);
        }
        if let Some(threads) = other.threads {
            self.threads = Some(threads);
        }
    }
}

/// Read and parse a YAML config layer at `path`. Returns `Ok(None)` when
/// the file does not exist (an absent optional layer is not an error).
/// Malformed YAML is reported as `DatagovError::InvalidArgs`, which maps
/// to exit code 2.
fn read_layer(path: &Path) -> Result<Option<Config>, DatagovError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let config: Config = serde_yaml::from_str(&contents).map_err(|source| {
                DatagovError::invalid_args(
                    format!("malformed config at {}: {source}", path.display()),
                    format!(
                        "fix the YAML syntax in {} (see PRD §28 for the expected shape)",
                        path.display()
                    ),
                )
            })?;
            Ok(Some(config))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(DatagovError::invalid_args(
            format!("cannot read config at {}: {err}", path.display()),
            format!("check permissions on {}", path.display()),
        )),
    }
}

fn apply_env_overrides(config: &mut Config, env: &BTreeMap<String, String>) {
    if let Some(value) = env.get("DATAGOV_OUTPUT")
        && let Ok(format) = value.parse::<OutputFormat>()
    {
        config.output = Some(format);
    }
    if let Some(value) = env.get("DATAGOV_THREADS")
        && let Ok(threads) = value.parse::<usize>()
    {
        config.threads = Some(threads);
    }
}

/// Resolve `Config` per the PRD §28 precedence chain (highest priority
/// last, so it overwrites earlier layers): built-in defaults, then
/// `~/.config/datagov/config.yaml`, then
/// `$XDG_CONFIG_HOME/datagov/config.yaml`, then `./.datagov/config.yaml`,
/// then `./datagov.yaml`, then `DATAGOV_*` environment variables. CLI
/// flags are the highest priority of all and are applied by the caller on
/// top of the returned `Config` — this function never sees them.
pub fn load(start_dir: &Path, env: &BTreeMap<String, String>) -> Result<Config, DatagovError> {
    let mut config = Config::default();

    if let Some(home) = env.get("HOME") {
        let path = PathBuf::from(home).join(".config/datagov/config.yaml");
        if let Some(layer) = read_layer(&path)? {
            config.merge_from(layer);
        }
    }

    if let Some(xdg) = env.get("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg).join("datagov/config.yaml");
        if let Some(layer) = read_layer(&path)? {
            config.merge_from(layer);
        }
    }

    if let Some(layer) = read_layer(&start_dir.join(".datagov/config.yaml"))? {
        config.merge_from(layer);
    }

    if let Some(layer) = read_layer(&start_dir.join("datagov.yaml"))? {
        config.merge_from(layer);
    }

    apply_env_overrides(&mut config, env);

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn env_with(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn defaults_when_nothing_present() {
        let dir = tempdir().unwrap();
        let config = load(dir.path(), &BTreeMap::new()).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn project_datagov_yaml_alone() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("datagov.yaml"),
            "output: json\nthreads: 4\n",
        )
        .unwrap();
        let config = load(dir.path(), &BTreeMap::new()).unwrap();
        assert_eq!(config.output, Some(OutputFormat::Json));
        assert_eq!(config.threads, Some(4));
    }

    #[test]
    fn project_dot_datagov_dir_alone() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".datagov")).unwrap();
        fs::write(dir.path().join(".datagov/config.yaml"), "output: table\n").unwrap();
        let config = load(dir.path(), &BTreeMap::new()).unwrap();
        assert_eq!(config.output, Some(OutputFormat::Table));
    }

    #[test]
    fn user_config_home_alone() {
        let dir = tempdir().unwrap();
        let home = tempdir().unwrap();
        fs::create_dir_all(home.path().join(".config/datagov")).unwrap();
        fs::write(
            home.path().join(".config/datagov/config.yaml"),
            "threads: 2\n",
        )
        .unwrap();
        let env = env_with(&[("HOME", home.path().to_str().unwrap())]);
        let config = load(dir.path(), &env).unwrap();
        assert_eq!(config.threads, Some(2));
    }

    #[test]
    fn xdg_config_home_alone() {
        let dir = tempdir().unwrap();
        let xdg = tempdir().unwrap();
        fs::create_dir_all(xdg.path().join("datagov")).unwrap();
        fs::write(xdg.path().join("datagov/config.yaml"), "threads: 8\n").unwrap();
        let env = env_with(&[("XDG_CONFIG_HOME", xdg.path().to_str().unwrap())]);
        let config = load(dir.path(), &env).unwrap();
        assert_eq!(config.threads, Some(8));
    }

    #[test]
    fn env_var_alone() {
        let dir = tempdir().unwrap();
        let env = env_with(&[("DATAGOV_OUTPUT", "json")]);
        let config = load(dir.path(), &env).unwrap();
        assert_eq!(config.output, Some(OutputFormat::Json));
    }

    #[test]
    fn override_order_env_beats_project_beats_dot_datagov_beats_xdg_beats_home() {
        let dir = tempdir().unwrap();
        let home = tempdir().unwrap();
        let xdg = tempdir().unwrap();

        fs::create_dir_all(home.path().join(".config/datagov")).unwrap();
        fs::write(
            home.path().join(".config/datagov/config.yaml"),
            "threads: 1\noutput: table\n",
        )
        .unwrap();

        fs::create_dir_all(xdg.path().join("datagov")).unwrap();
        fs::write(xdg.path().join("datagov/config.yaml"), "threads: 2\n").unwrap();

        fs::create_dir_all(dir.path().join(".datagov")).unwrap();
        fs::write(dir.path().join(".datagov/config.yaml"), "threads: 3\n").unwrap();

        fs::write(dir.path().join("datagov.yaml"), "threads: 4\n").unwrap();

        let env = env_with(&[
            ("HOME", home.path().to_str().unwrap()),
            ("XDG_CONFIG_HOME", xdg.path().to_str().unwrap()),
            ("DATAGOV_OUTPUT", "json"),
        ]);

        let config = load(dir.path(), &env).unwrap();
        // threads: highest priority present is ./datagov.yaml (4).
        assert_eq!(config.threads, Some(4));
        // output: env DATAGOV_OUTPUT beats the home-level "table".
        assert_eq!(config.output, Some(OutputFormat::Json));
    }

    #[test]
    fn malformed_yaml_maps_to_exit_code_2() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("datagov.yaml"),
            "output: [this is not valid\n",
        )
        .unwrap();
        let err = load(dir.path(), &BTreeMap::new()).unwrap_err();
        assert_eq!(err.exit_code(), crate::exit::ExitCode::InvalidArgs);
    }

    #[test]
    fn secrets_field_is_not_part_of_the_surface() {
        // Config has exactly output/threads — no credential-shaped field
        // exists to accidentally read from a project file.
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("datagov.yaml"),
            "output: json\nthreads: 1\napi_key: should-be-ignored\n",
        )
        .unwrap();
        let config = load(dir.path(), &BTreeMap::new()).unwrap();
        assert_eq!(config.output, Some(OutputFormat::Json));
        assert_eq!(config.threads, Some(1));
    }
}
