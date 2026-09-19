{
  description = "Tower middleware and utilities for HTTP clients.";

  inputs = {
    # Keep the compiler and Nix package set on the same stable channel.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-parts.url = "github:hercules-ci/flake-parts";

    # Reuse the shared Rust overlay and project-source helper.
    nix-devtools = {
      url = "github:alekseysidorov/nix-devtools";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
      inputs.treefmt-nix.follows = "treefmt-nix";
      inputs.rust-advisory-db.follows = "rust-advisory-db";
    };

    rust-advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake
      {
        inherit inputs;
        # The hook module currently resolves pkgs from these two provider values.
        specialArgs.localInputs = {
          inherit (inputs) nixpkgs;
          self = inputs.nix-devtools;
        };
      }
      (
        { ... }:
        let
          inherit (inputs.nixpkgs) lib;
        in
        {
          systems = lib.systems.flakeExposed;
          imports = [
            inputs.treefmt-nix.flakeModule
            inputs.nix-devtools.flakeModule
          ];

          perSystem =
            { system, ... }:
            let
              # Extend this flake's package set locally; rust-bin is an implementation detail.
              pkgs = inputs.nixpkgs.legacyPackages.${system}.extend inputs.nix-devtools.overlays.default;

              # Keep the minimum compiler explicit while following the current stable channel.
              rustVersions = {
                msrv = "1.92.0";
                stable = "1.96.0";
              };

              # Checks use MSRV; development follows stable, and nightly supplies only rustfmt.
              rustToolchains = {
                stable = pkgs.rust-bin.stable.${rustVersions.stable}.default.override {
                  extensions = [
                    "clippy"
                    "rust-src"
                    "rustfmt"
                  ];
                };
                msrv = pkgs.rust-bin.stable.${rustVersions.msrv}.default;
                nightly = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.rustfmt);
              };

              # Respect .gitignore while retaining all project files needed by Cargo.
              src = pkgs.projectSource { projectRoot = ./.; };

              # Reuse nix-devtools' vendoring and shared Crane artifacts for project checks.
              rustDev = pkgs.mkRustDevHelpers {
                inherit pkgs src;
                toolchain = rustToolchains.msrv;
              };
            in
            {
              packages = {
                # Cargo package itself.
                default = rustDev.craneLib.buildPackage {
                  inherit src;
                  strictDeps = true;
                  cargoVendorDir = rustDev.craneLib.vendorCargoDeps { inherit src; };
                  cargoArtifacts = rustDev.cargoArtifacts;
                };

                check-cargo-semver = pkgs.writeNushellApplication {
                  name = "check-cargo-semver";
                  runtimeInputs = [
                    rustToolchains.stable
                    pkgs.cargo-semver-checks
                  ];
                  text = ''
                    def main [...args: string] {
                      ^cargo semver-checks --workspace ...$args
                    }
                  '';
                };

                check-cargo-publish = pkgs.writeNushellApplication {
                  name = "check-cargo-publish";
                  runtimeInputs = [ rustToolchains.stable ];
                  text = ''
                    def main [...args: string] {
                      ^cargo publish --workspace --dry-run --allow-dirty ...$args
                    }
                  '';
                };
              };

              checks = {
                test = rustDev.checks.nextest "--workspace --all-targets --no-default-features";
                test-all-features = rustDev.checks.nextest "--workspace --all-targets --all-features";
                clippy = rustDev.checks.clippy "--workspace --all-targets --all-features -- -D warnings";
                doc = rustDev.checks.doc "--workspace --all-features --no-deps";
                doctest = rustDev.checks.test "--doc --workspace --all-features";
                audit = rustDev.checks.audit "";
              };

              devShells.default = pkgs.mkShell {
                packages = [
                  rustToolchains.stable
                  pkgs.cargo-audit
                  pkgs.cargo-nextest
                ];
              };

              treefmt = {
                projectRootFile = "flake.nix";
                programs = {
                  nixfmt.enable = true;
                  rustfmt = {
                    enable = true;
                    package = rustToolchains.nightly;
                  };
                  taplo.enable = true;
                };
              };

              # Install explicitly with `nix run .#install-git-hooks`.
              gitHooks = {
                pre-commit = pkgs.writeNushellScript "pre-commit" ''
                  print "⚡️ Running pre-commit checks..."
                  nix fmt -- --fail-on-change
                '';
                pre-push = pkgs.writeNushellScript "pre-push" ''
                  print "⚡️ Running flake checks..."
                  nix flake check -L
                  print "⚡️ Running semver checks..."
                  nix run .#check-cargo-semver -L
                  print "⚡️ Running cargo publish compatibility checks..."
                  nix run .#check-cargo-publish -L
                '';
              };
            };
        }
      );
}
