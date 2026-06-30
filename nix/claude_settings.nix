builtins.toJSON {
  permissions = {
    allow = [
      # Nix
      "Bash(nix flake check*)"
      "Bash(nix eval*)"
      "Bash(nixos-rebuild dry-build*)"
      "Bash(statix check*)"
      "Bash(deadnix*)"
      "Bash(just*)"
      "Bash(nix build --dry-run*)"
      "Bash(nix search nixpkgs*)"
      "Bash(curl -s https://search.nixos.org*)"

      # Rust
      "Bash(cargo check*)"
      "Bash(cargo clippy*)"
      "Bash(cargo nextest run*)"
      "Bash(cargo test*)"
      "Bash(cargo tree*)"
      "Bash(cargo machete*)"
      "Bash(cargo deny check*)"
      "Bash(cargo audit*)"
    ];
  };
  enabledPlugins = {
    "claude-md-management@claude-plugins-official" = true;
    "superpowers@claude-plugins-official" = true;
    "context7@claude-plugins-official" = true;
    "code-review@claude-plugins-official" = true;
    "code-simplifier@claude-plugins-official" = true;
    "github@claude-plugins-official" = true;
    "rust-analyzer-lsp@claude-plugins-official" = true;
  };
}
