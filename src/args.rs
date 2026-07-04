use std::{process::Command, str::FromStr};

use clap::Parser;
use ratatui::style::Color;
use url::Url;

use crate::tui::ui::Tab;

/// Argument parser powered by [`clap`].
#[derive(Clone, Debug, Default, Parser)]
#[clap(
    version,
    author = clap::crate_authors!("\n"),
    about,
    rename_all_env = "screaming-snake",
    help_template = "\
{before-help}{name} {version}
{author-with-newline}{about-with-newline}
{usage-heading}
  {usage}

{all-args}{after-help}
",
)]
pub struct Args {
    /// Path or URL to the Nixos configuration.
    #[arg(env = "SOURCE",short = 'c', long, value_name = "PATH | URL",value_parser = verify_flake )]
    pub config: String,

    /// Accent color of the application.
    #[arg(env, long, value_name = "COLOR")]
    pub accent_color: Option<Color>,

    /// The initial application tab to open.
    #[arg(short, long, default_value = "general")]
    pub tab: Tab,

    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

fn verify_flake(s: &str) -> Result<String, String> {
    let s = match url::Url::from_str(s) {
        Ok(url) => to_flake_ref(&url),
        Err(_) => s.to_string(),
    };
    let output = Command::new("nix")
        .args(["flake", "metadata", "--json", "--no-write-lock-file", &s])
        .output()
        .map_err(|e| format!("failed to run nix: {e}"))?;

    if output.status.success() {
        Ok(s)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

/// Convert a user-supplied URL into a Nix flake reference.
fn to_flake_ref(url: &Url) -> String {
    match url.scheme() {
        // Already a flake-ref scheme, pass through.
        "github" | "gitlab" | "sourcehut" | "path" | "flake" => url.to_string(),
        s if s.starts_with("git+") || s.starts_with("tarball+") => url.to_string(),

        "https" | "http" | "ssh" => {
            let host = url.host_str().unwrap_or_default();
            // github.com/owner/repo -> github:owner/repo
            if host == "github.com" || host == "gitlab.com" {
                let mut segs = url.path_segments().into_iter().flatten();
                if let (Some(owner), Some(repo)) = (segs.next(), segs.next()) {
                    let repo = repo.trim_end_matches(".git");
                    let scheme = if host == "github.com" {
                        "github"
                    } else {
                        "gitlab"
                    };
                    return format!("{scheme}:{owner}/{repo}");
                }
            }
            // Anything else: assume it's a git remote.
            format!("git+{url}")
        }
        _ => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn test_args() {
        Args::command().debug_assert();
    }

    #[test]
    fn test_verify_flake_github_url() {
        assert!(verify_flake("https://github.com/wallago/nix-config").is_ok());
    }

    #[test]
    fn test_config_is_required() {
        assert!(Args::try_parse_from(["flamingo"]).is_err());
    }

    #[test]
    fn test_config_arg_valid_flake() {
        let dir = tempdir::TempDir::new("flamingo-test").unwrap();
        std::fs::write(dir.path().join("flake.nix"), "{ outputs = _: { }; }").unwrap();

        let args =
            Args::try_parse_from(["flamingo", "--config", dir.path().to_str().unwrap()]).unwrap();
        assert_eq!(args.config, dir.path().to_str().unwrap());
    }
}
