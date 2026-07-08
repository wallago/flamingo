use std::process::Command;

use crate::app::Flake;

impl Flake {
    /// Runs git in the source tree, returning trimmed stdout on success.
    fn git_output(source: &str, args: &[&str]) -> Option<String> {
        let out = Command::new("git")
            .args(["-C", source])
            .args(args)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Collects display metadata: flake description, input count, and the
    /// git state of the source tree. Purely informative, so failures
    /// (no git repo, no description) just leave the fields empty.
    pub fn extract_metadata(&mut self) {
        if let Ok(out) = Command::new("nix")
            .args(["flake", "metadata", &self.source, "--json"])
            .output()
            && out.status.success()
            && let Ok(metadata) = serde_json::from_slice::<serde_json::Value>(&out.stdout)
        {
            self.description = metadata["description"].as_str().map(str::to_string);
            let root = metadata["locks"]["root"].as_str().unwrap_or("root");
            self.input_count = metadata["locks"]["nodes"][root]["inputs"]
                .as_object()
                .map(|inputs| inputs.len())
                .unwrap_or_default();
        }
        self.branch = Self::git_output(&self.source, &["rev-parse", "--abbrev-ref", "HEAD"]);
        self.commit = Self::git_output(&self.source, &["rev-parse", "--short", "HEAD"]);
        self.dirty = Self::git_output(&self.source, &["status", "--porcelain"])
            .is_some_and(|status| !status.is_empty());
    }
}
