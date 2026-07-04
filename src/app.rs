use crate::{
    error::{Error, Result},
    module::Module,
};
use std::process::Command;
use tempdir::TempDir;
use url::Url;

/// Nixos Config.
#[derive(Debug)]
pub struct Config {
    /// Path of config.
    pub source: String,
    /// Tempdir to achieve all operations.
    temp: TempDir,
    /// Config modules.
    pub modules: Vec<Module>,
}

impl Config {
    /// Constructs a new instance.
    pub fn new(source: &str) -> Result<Self> {
        let temp = TempDir::new("flamingo")?;
        Ok(Self {
            source: source.to_string(),
            temp,
            modules: None,
        })
    }

    /// Extracts modules.
    pub fn extract_modules(&mut self) -> Result<()> {
        let out = Command::new("nix")
            .args([
                "eval",
                &format!("{}#nixosModules", self.source),
                "--apply",
                "builtins.attrNames",
                "--json",
            ])
            .output()?;

        let names: Vec<String> = serde_json::from_slice(&out.stdout)?;
        self.available_modules = Some(names);
        Ok(())
        // nix eval .#nixosModules --apply builtins.attrNames --json | jq -r '.[]'
        // nix eval .#homeModules --apply builtins.attrNames --json | jq -r '.[]'
        // nix eval github:wallago/nix-config#nixosModules --apply builtins.attrNames --json
    }
}
