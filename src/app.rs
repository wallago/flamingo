use crate::{
    error::{Error, Result},
    tui::event::Event,
};
use heh::app::Application as Heh;
use heh::decoder::Encoding;
use ratatui::text::Line;
use std::{
    env,
    fmt::{self, Debug, Formatter},
    path::PathBuf,
    process::Command,
    sync::mpsc,
    thread,
};
use tempdir::TempDir;
use url::Url;

/// Nixos Config.
pub struct Config {
    /// Path of config.
    pub path: String,
    /// Tempdir to achieve all operations.
    temp: TempDir,
}

impl Config {
    /// Constructs a new instance.
    pub fn new(local_path: Option<String>, git_repo_url: Option<Url>) -> Result<Self> {
        let temp = TempDir::new("flamingo")?;
        let path: String = match (local_path, git_repo_url) {
            (Some(local_path), None) => local_path,
            (None, Some(url)) => url.to_string(),
            _ => {
                return Err(Error::ArgsError(
                    "local path or github repo URL not fit".into(),
                ));
            }
        };
        Ok(Self { path, temp })
    }

    /// Extracts modules.
    pub fn extract_modules(&self) -> Result<Vec<String>> {
        let out = Command::new("nix")
            .args([
                "eval",
                &format!("{}#nixosModules", self.path),
                "--apply",
                "builtins.attrNames",
                "--json",
            ])
            .output()?;

        let names: Vec<String> = serde_json::from_slice(&out.stdout)?;
        Ok(names)
        // nix eval .#nixosModules --apply builtins.attrNames --json | jq -r '.[]'
        // nix eval .#homeModules --apply builtins.attrNames --json | jq -r '.[]'
        // nix eval github:wallago/nix-config#nixosModules --apply builtins.attrNames --json
    }
}
