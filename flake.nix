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
      in
      {
        # for `nix fmt`
        formatter = treefmt.wrapper;

        # for `nix flake check`
        checks = {
          # Check formatting
          formatting = treefmt.check self;

          # Run tests with MSRV toolchain
          tests = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoNextestExtraArgs = "--workspace --all-targets --no-default-features";
            }
          );

          # Run tests with all features
          tests-all-features = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoNextestExtraArgs = "--workspace --all-targets --all-features";
            }
          );

          # Run clippy
          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--workspace --all --all-targets --all-features -- --deny warnings";
            }
          );

          # Run doc tests
          doc-tests = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoTestExtraArgs = "--workspace --doc --all-features";
            }
          );

          # Check documentation builds
          doc = craneLib.cargoDoc (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoDocExtraArgs = "--workspace --no-deps --all-features";
            }
          );
        };

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
