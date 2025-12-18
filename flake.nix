# Nix flake for tower-http-client development and CI
#
# Usage:
#   nix flake check              - Run all checks (formatting, clippy, tests, docs)
#   nix fmt                      - Format code
#   nix build .#check-clippy     - Run only clippy
#   nix build .#check-tests      - Run only tests (no default features)
#   nix build .#check-tests-all  - Run tests with all features
#   nix build .#check-doc        - Check documentation builds
#   nix build .#check-doc-tests  - Run doc tests
#   nix build .#check-fmt        - Check formatting
#   nix run .#benchmarks         - Run benchmarks
#   nix run .#git-install-hooks  - Install git hooks (pre-commit: fmt, pre-push: full checks)
#   nix develop                  - Enter development shell with stable Rust
#   nix develop .#nightly        - Enter development shell with nightly Rust
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    fenix.url = "github:nix-community/fenix/monthly";
    crane.url = "github:ipetkov/crane";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
      crane,
      treefmt-nix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        # Common nix packages
        pkgs = nixpkgs.legacyPackages.${system};

        # Fenix Rust toolchains
        fenixPackage = fenix.packages.${system};

        # Minimum supported Rust version
        msrv = {
          name = "1.89.0";
          sha256 = "sha256-+9FmLhAOezBZCOziO0Qct1NOrfpjNsXxc/8I0c7BdKE=";
        };

        rustToolchains = {
          stable = fenixPackage.stable.completeToolchain;
          msrv = (fenixPackage.fromToolchainName msrv).defaultToolchain;
          nightly = fenixPackage.complete.withComponents [ "rustfmt" ];
        };

        # Crane library for building Rust packages
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchains.msrv;

        # Eval the treefmt configuration
        treefmtConfig = {
          projectRootFile = "flake.nix";

          programs = {
            nixfmt.enable = true;
            rustfmt = {
              enable = true;
              package = rustToolchains.nightly;
            };
            beautysh.enable = true;
            deno.enable = true;
            taplo.enable = true;
          };
        };
        treefmt = (treefmt-nix.lib.evalModule pkgs treefmtConfig).config.build;

        # Runtime inputs for all CI scripts
        runtimeInputs = with pkgs; [
          cargo-nextest
          openssl
          pkg-config
        ];

        # Source filtering for crane
        src = craneLib.path ./.;

        # Common arguments for all crane builds
        commonArgs = {
          inherit src;
          pname = "tower-http-client-workspace";
          version = "0.5.4";
          strictDeps = true;
          nativeBuildInputs = runtimeInputs;
          cargoVendorDir = craneLib.vendorCargoDeps {
            inherit src;
          };
        };

        # Build dependencies only (for caching)
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the workspace
        workspace = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
          }
        );

        # Helper function to create a check with common args
        mkCheck = builder: extraArgs: builder (commonArgs // { inherit cargoArtifacts; } // extraArgs);

        # Define checks that can be reused in packages
        checks = {
          # Check formatting
          formatting = treefmt.check self;

          # Run tests with MSRV toolchain
          tests = mkCheck craneLib.cargoNextest {
            cargoNextestExtraArgs = "--workspace --all-targets --no-default-features";
          };

          # Run tests with all features
          tests-all-features = mkCheck craneLib.cargoNextest {
            cargoNextestExtraArgs = "--workspace --all-targets --all-features";
          };

          # Run clippy
          clippy = mkCheck craneLib.cargoClippy {
            cargoClippyExtraArgs = "--workspace --all --all-targets --all-features -- --deny warnings";
          };

          # Run doc tests
          doc-tests = mkCheck craneLib.cargoTest {
            cargoTestExtraArgs = "--workspace --doc --all-features";
          };

          # Check documentation builds
          doc = mkCheck craneLib.cargoDoc {
            cargoDocExtraArgs = "--workspace --no-deps --all-features";
          };
        };
      in
      {
        # for `nix fmt`
        formatter = treefmt.wrapper;

        # for `nix flake check`
        inherit checks;

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = runtimeInputs ++ [
            rustToolchains.stable
            treefmt.wrapper
          ];
        };

        # Nightly compiler to run miri tests
        devShells.nightly = pkgs.mkShell {
          nativeBuildInputs = [ rustToolchains.nightly ];
        };

        packages = {
          # Benchmarks package for local performance testing
          benchmarks = pkgs.writeShellApplication {
            name = "run-benchmarks";
            runtimeInputs = [ rustToolchains.stable ] ++ runtimeInputs;
            text = ''
              cargo bench --workspace --all-features
            '';
          };

          # Convenience wrappers to run specific checks
          check-clippy = checks.clippy;
          check-tests = checks.tests;
          check-tests-all = checks.tests-all-features;
          check-doc = checks.doc;
          check-doc-tests = checks.doc-tests;
          check-fmt = checks.formatting;

          git-install-hooks = pkgs.writeShellApplication {
            name = "install-git-hooks";
            text = ''
              echo "-> Installing pre-commit hook"
              echo "nix fmt -- --fail-on-change" >> "$PWD/.git/hooks/pre-commit"
              chmod +x "$PWD/.git/hooks/pre-commit"

              echo "-> Installing pre-push hook"
              echo "nix flake check" >> "$PWD/.git/hooks/pre-push"
              chmod +x "$PWD/.git/hooks/pre-push"
            '';
          };
        };
      }
    );
}
