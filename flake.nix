{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    rust-overlay.url = "github:oxalica/rust-overlay";
    claude-code.url = "github:sadjow/claude-code-nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      naersk,
      claude-code,
      ...
    }:
    # ── System-agnostic outputs (modules) live out here ──
    {
      # nixosModules.default = import ./nix/module.nix self;
      # homeModules.default = import ./nix/hm-module.nix self;
    }
    # ── Then merge the per-system outputs onto it ──
    // flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        };

        # ── Toolchain ─────────────────────────────────────────────
        rust = pkgs.rust-bin.nightly.latest.default;

        naersk' = pkgs.callPackage naersk {
          cargo = rust;
          rustc = rust;
        };

        # ── Build helper ──────────────────────────────────────────
        buildApp =
          { release }:
          pkgs.callPackage ./nix/package.nix {
            inherit naersk' release;
            src = ./.;
          };

        # ── Claude Settings ─────────────────────────────────────
        claude = claude-code.packages.${system}.default;
        claudeLocalSettings = import ./nix/claude_settings.nix;

        # ── Tooling shared by the dev shell and CI ───────────────
        ciTools = with pkgs; [
          rust

          # rust tooling
          cargo-nextest
          cargo-deny
          cargo-audit
          cargo-machete
          cargo-edit

          # repo tooling
          typos
          committed
          git-cliff

          # nix tooling
          nixfmt
          statix
          deadnix

          # crate deps
        ];
      in
      {
        # ── Packages ──────────────────────────────────────────────
        packages = rec {
          flamingo = buildApp { release = true; };
          flamingo-debug = buildApp { release = false; };
          default = flamingo;
        };

        # ── Checks (nix flake check) ─────────────────────────────
        checks.check = self.packages.${system}.flamingo-debug;

        # ── Dev Shell (nix develop) ──────────────────────────────
        devShells.default = pkgs.mkShell {
          buildInputs =
            ciTools
            ++ (with pkgs; [
              rust-analyzer
              just
              claude
              nodejs
            ]);
          shellHook = ''
            mkdir -p .claude
            echo '${claudeLocalSettings}' > .claude/settings.local.json
          '';
        };

        # ── CI Shell (nix develop .#ci) ──────────────────────────
        # Lean: just the toolchain + checks, no editor/claude/shellHook.
        devShells.ci = pkgs.mkShell {
          buildInputs = ciTools;
        };
      }
    );
}
