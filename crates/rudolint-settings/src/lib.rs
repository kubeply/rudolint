//! Resolved settings after config discovery and CLI overrides.

use std::path::{Path, PathBuf};

use anyhow::Result;
use rudolint_config::Config;

/// Fully resolved settings used by the linter.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// Loaded configuration values.
    pub config: Config,
    /// Path to the configuration file that was loaded, if any.
    pub config_path: Option<PathBuf>,
}

/// Inputs used to resolve effective settings.
#[derive(Debug, Clone, Default)]
pub struct SettingsOptions {
    /// Explicit configuration file path supplied by the caller.
    pub explicit_config: Option<PathBuf>,
    /// Whether configuration loading and discovery are disabled.
    pub no_config: bool,
    /// Paths used as starting points for configuration discovery.
    pub search_starts: Vec<PathBuf>,
}

/// Resolves effective settings from discovery and override options.
pub fn resolve(options: &SettingsOptions) -> Result<Settings> {
    if options.no_config {
        return Ok(Settings::default());
    }

    if let Some(path) = &options.explicit_config {
        return Ok(Settings {
            config: Config::load(Some(path))?,
            config_path: Some(path.clone()),
        });
    }

    let config_path = rudolint_config::discover(&options.search_starts)?;
    let config = Config::load(config_path.as_deref())?;
    Ok(Settings {
        config,
        config_path,
    })
}

impl SettingsOptions {
    /// Sets an explicit configuration file path.
    pub fn with_explicit_config(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_config = Some(path.into());
        self
    }

    /// Disables configuration loading and discovery.
    pub fn without_config(mut self) -> Self {
        self.no_config = true;
        self
    }

    /// Sets the start paths used for configuration discovery.
    pub fn with_search_starts(mut self, starts: impl IntoIterator<Item = PathBuf>) -> Self {
        self.search_starts = starts.into_iter().collect();
        self
    }
}

/// Resolves settings from simple parts without constructing `SettingsOptions` manually.
pub fn resolve_from_parts(
    explicit_config: Option<&Path>,
    no_config: bool,
    search_starts: Vec<PathBuf>,
) -> Result<Settings> {
    resolve(&SettingsOptions {
        explicit_config: explicit_config.map(Path::to_path_buf),
        no_config,
        search_starts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_config_wins() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        let discovered = temp.path().join(".rudolint.yaml");
        let explicit = temp.path().join("explicit.yaml");
        std::fs::write(&discovered, "ignore:\n  - DL3000\n").expect("config should be written");
        std::fs::write(&explicit, "ignore:\n  - DL3007\n").expect("config should be written");

        let settings = resolve_from_parts(Some(&explicit), false, vec![temp.path().to_path_buf()])
            .expect("settings should resolve");

        assert_eq!(settings.config_path, Some(explicit));
        assert!(settings.config.ignores("DL3007"));
        assert!(!settings.config.ignores("DL3000"));
    }

    #[test]
    fn no_config_skips_discovery() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - DL3000\n")
            .expect("config should be written");

        let settings = resolve_from_parts(None, true, vec![temp.path().to_path_buf()])
            .expect("settings should resolve");

        assert!(settings.config_path.is_none());
        assert!(!settings.config.ignores("DL3000"));
    }

    #[test]
    fn discovers_dot_config() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        let nested = temp.path().join("service");
        std::fs::create_dir(&nested).expect("nested dir should be created");
        let config = temp.path().join(".rudolint.yaml");
        std::fs::write(&config, "ignore:\n  - DL3000\n").expect("config should be written");

        let settings =
            resolve_from_parts(None, false, vec![nested]).expect("settings should resolve");

        assert_eq!(settings.config_path, Some(config.canonicalize().unwrap()));
        assert!(settings.config.ignores("DL3000"));
    }

    #[test]
    fn snapshots_config_precedence_matrix() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        let nested = temp.path().join("service");
        std::fs::create_dir(&nested).expect("nested dir should be created");
        let discovered = temp.path().join(".rudolint.yaml");
        let explicit = temp.path().join("explicit.yaml");

        std::fs::write(&discovered, "ignore: [DL3000\n")
            .expect("invalid discovered config should be written");
        std::fs::write(&explicit, "ignore:\n  - DL3007\n")
            .expect("explicit config should be written");

        let explicit_settings = resolve_from_parts(Some(&explicit), false, vec![nested.clone()])
            .expect("explicit config should win before discovery");
        let no_config_settings = resolve_from_parts(None, true, vec![nested.clone()])
            .expect("no-config should skip discovery");
        let discovered_error = resolve_from_parts(None, false, vec![nested])
            .expect_err("invalid discovered config should fail without override")
            .to_string();

        let snapshot = serde_json::json!({
            "explicit_config": {
                "path": explicit_settings
                    .config_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str()),
                "ignores_dl3007": explicit_settings.config.ignores("DL3007"),
                "ignores_dl3000": explicit_settings.config.ignores("DL3000"),
            },
            "no_config": {
                "path": no_config_settings
                    .config_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str()),
                "ignores_dl3000": no_config_settings.config.ignores("DL3000"),
            },
            "discovered_config_error": discovered_error.contains(".rudolint.yaml"),
        });
        insta::assert_json_snapshot!("config_precedence_matrix", snapshot);
    }
}
