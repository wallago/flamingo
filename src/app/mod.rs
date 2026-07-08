use crate::{config::AppConfig, error::Result, module::Module};

/// Module extraction through `nix eval`.
mod extract;
/// Flake description and git state for the info panel.
mod metadata;
/// Textual recovery of what the eval can't see.
mod recover;
/// Tree nesting, added-flag cascading and export selection.
mod tree;

/// Nixos configuration.
#[derive(Debug)]
pub struct Flake {
    /// Path of flake.
    pub source: String,
    /// Nixos modules.
    pub nixos_modules: Vec<Module>,
    /// HomeManager modules.
    pub home_modules: Vec<Module>,
    /// Flake description.
    pub description: Option<String>,
    /// Number of direct flake inputs.
    pub input_count: usize,
    /// Git branch of the source tree.
    pub branch: Option<String>,
    /// Short commit hash of the source tree.
    pub commit: Option<String>,
    /// Whether the source tree has uncommitted changes.
    pub dirty: bool,
}

impl Flake {
    /// Constructs a new instance.
    pub fn new(source: &str) -> Result<Self> {
        Ok(Self {
            source: source.to_string(),
            nixos_modules: Vec::new(),
            home_modules: Vec::new(),
            description: None,
            input_count: 0,
            branch: None,
            commit: None,
            dirty: false,
        })
    }

    /// Loads a flake: extracts modules, recovers what the eval can't see,
    /// and gathers display metadata.
    pub fn load(source: &str, config: &AppConfig) -> Result<Self> {
        let mut flake = Self::new(source)?;
        flake.extract_modules(config)?;
        flake.extract_metadata();
        Ok(flake)
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use crate::app::Flake;

    #[fixture]
    fn flake() -> Flake {
        Flake::new("https://github.com/wallago/nix-config").expect("failed to create flake fixture")
    }

    #[rstest]
    fn should_process_flake(flake: Flake) {
        assert_eq!(flake.source, "https://github.com/wallago/nix-config");
    }
}
