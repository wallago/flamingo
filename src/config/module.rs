use serde::Deserialize;

/// Configurable Nixos and HomeManager Modules.
#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Modules {
    /// Nixos Modules.
    pub nixos: Vec<String>,
    /// HomeManager Modules.
    pub home: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_deserializes_toml_list() {
        #[derive(Deserialize)]
        struct Wrapper {
            nixos: Vec<String>,
            home: Vec<String>,
        }
        let modules: Wrapper = toml::from_str("nixos = [\"test\"]\nhome = [\"test\"]").unwrap();
        assert_eq!(modules.nixos.len(), 1);
        assert_eq!(modules.home.len(), 1);
        assert!(toml::from_str::<Wrapper>("nixos = \"test\"\nhome = \"test\"").is_err());
    }
}
