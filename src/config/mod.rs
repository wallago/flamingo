//! Application configuration.

use std::path::Path;

use serde::Deserialize;

use keybinding::Keybindings;

use crate::{
    config::module::Modules,
    error::{Error, Result},
};

pub mod keybinding;
mod module;

/// Application configuration loaded from `config.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    /// Key bindings.
    pub keybindings: Keybindings,
    /// Nixos and HomeManager Module names to mark as added on startup.
    pub modules: Modules,
}

impl AppConfig {
    /// Loads the config: explicit `--config` path, else
    /// `$XDG_CONFIG_HOME/flamingo/config.toml` if present, else defaults.
    pub fn load(cli_path: Option<&Path>) -> Result<Self> {
        if let Some(path) = cli_path {
            return Self::from_file(path);
        }
        match dirs::config_dir().map(|dir| dir.join("flamingo/config.toml")) {
            Some(path) if path.exists() => Self::from_file(&path),
            _ => Ok(Self::default()),
        }
    }

    /// Reads and parses a config file; any failure is a hard error.
    pub fn from_file(path: &Path) -> Result<Self> {
        // Get raw content
        let raw = std::fs::read_to_string(path)
            .map_err(|error| Error::AppConfigError(format!("{}: {error}", path.display())))?;
        // Deserialize content in TOML format
        toml::from_str(&raw)
            .map_err(|error| Error::AppConfigError(format!("{}: {error}", path.display())))
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent};

    use super::*;

    #[test]
    fn should_parse_config_toml() {
        let config =
            AppConfig::from_file(std::path::Path::new("config.toml")).expect("example config");
        assert!(
            config
                .keybindings
                .quit
                .matches(&KeyEvent::from(KeyCode::Char('q')))
        );
        assert_eq!(config.modules.nixos, vec!["general".to_string()]);
        assert_eq!(config.modules.home, vec!["general".to_string()]);
    }

    #[test]
    fn load_explicit_path_missing_is_hard_error() {
        assert!(AppConfig::load(Some(std::path::Path::new("/nonexistent/config.toml"))).is_err());
    }
}
