use crate::module::Module;

use regex::Regex;
use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Non-fatal problem discovered while trimming a config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// A path literal contains `${...}` interpolation and cannot be followed.
    Dynamic {
        /// File containing the literal.
        file: PathBuf,
        /// The literal text.
        literal: String,
    },
    /// A path literal points outside the source repo.
    OutsideRepo {
        /// File containing the literal.
        file: PathBuf,
        /// The literal text.
        literal: String,
    },
    /// A path literal does not resolve to an existing file.
    Missing {
        /// File containing the literal.
        file: PathBuf,
        /// The literal text.
        literal: String,
    },
    /// A `.nix` file could not be read.
    Unreadable {
        /// The unreadable file.
        file: PathBuf,
    },
    /// An added module has no resolvable source path and was skipped.
    UnresolvedModule {
        /// The module name.
        name: String,
    },
    /// The chosen hostname already exists in the source config.
    HostnameExists {
        /// The colliding hostname.
        hostname: String,
    },
    /// The nixpkgs release could not be determined; stateVersion left commented.
    NoStateVersion,
    /// No inputs were found in the source flake.nix; a default was generated.
    NoInputs,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dynamic { file, literal } => {
                write!(f, "dynamic path `{literal}` in {} skipped", file.display())
            }
            Self::OutsideRepo { file, literal } => {
                write!(
                    f,
                    "`{literal}` in {} points outside the repo",
                    file.display()
                )
            }
            Self::Missing { file, literal } => {
                write!(f, "`{literal}` in {} does not exist", file.display())
            }
            Self::Unreadable { file } => write!(f, "could not read {}", file.display()),
            Self::UnresolvedModule { name } => {
                write!(f, "module `{name}` has no resolvable source, skipped")
            }
            Self::HostnameExists { hostname } => {
                write!(f, "host `{hostname}` already exists in the source config")
            }
            Self::NoStateVersion => {
                write!(
                    f,
                    "could not detect nixpkgs release; stateVersion left commented"
                )
            }
            Self::NoInputs => {
                write!(
                    f,
                    "no inputs found in source flake.nix; default nixpkgs input generated"
                )
            }
        }
    }
}

/// A relative path literal found in Nix source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathLiteral {
    /// The literal text, e.g. `../common/base.nix`.
    pub text: String,
    /// Whether the literal is immediately followed by `${` interpolation.
    pub dynamic: bool,
}

/// Removes `#` line comments and `/* */` block comments from Nix source.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '#' => {
                for (_, c) in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if src[i..].starts_with("/*") => {
                chars.next(); // consume '*'
                let mut prev = ' ';
                for (_, c) in chars.by_ref() {
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Extracts relative path literals (`./…`, `../…`) from Nix source text.
pub fn path_literals(src: &str) -> Vec<PathLiteral> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"\.\.?/[A-Za-z0-9_\-./+@]*").expect("hardcoded regex is valid")
    });
    let stripped = strip_comments(src);
    re.find_iter(&stripped)
        .filter(|m| {
            // Reject matches glued to a preceding word or dot (`a./x`, `.../x`).
            !stripped[..m.start()]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '.' || c == '_')
        })
        .map(|m| PathLiteral {
            text: m.as_str().to_string(),
            dynamic: stripped[m.end()..].starts_with("${"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_literals_basic() {
        let found = path_literals("{ imports = [ ./a.nix ../common ]; }");
        assert_eq!(
            found,
            vec![
                PathLiteral {
                    text: "./a.nix".into(),
                    dynamic: false
                },
                PathLiteral {
                    text: "../common".into(),
                    dynamic: false
                },
            ]
        );
    }

    #[test]
    fn test_path_literals_dynamic_and_comments() {
        let src = "# see ./ignored.nix\n/* ./also.nix */\nsource = ./hosts/${name}/x.nix;";
        let found = path_literals(src);
        assert_eq!(found.len(), 1);
        assert!(found[0].dynamic);
        assert_eq!(found[0].text, "./hosts/");
    }

    #[test]
    fn test_path_literals_ignores_ellipsis_and_word_glued_dots() {
        assert!(path_literals("{ pkgs, ... }: {}").is_empty());
        // `.../x` : the leading dot glues to a previous dot — not a path.
        assert!(path_literals("a.../x").is_empty());
    }
}
