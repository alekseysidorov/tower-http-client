# Nix flake for tower-http-client development and CI
#
# Usage:
#   nix flake check              - Run all checks (formatting, clippy, tests, docs)
#   nix fmt                      - Format code
#
#   nix build .#check-clippy     - Run only clippy
#   nix build .#check-tests      - Run only tests (no default features)
#   nix build .#check-tests-all  - Run tests with all features
#   nix build .#check-doc        - Check documentation builds
#   nix build .#check-doc-tests  - Run doc tests
#   nix build .#check-fmt        - Check formatting
#
#   nix run .#benchmarks         - Run benchmarks
#   nix run .#check-semver       - Run semver compatibility checks (requires network)
#   nix run .#git-install-hooks  - Install git hooks (pre-commit: fmt, pre-push: checks + semver)
#
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

        # Common build inputs for all CI scripts
        buildInputs = with pkgs; [
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
          version = "0.6.0";
          strictDeps = true;
          nativeBuildInputs = buildInputs;
          cargoVendorDir = craneLib.vendorCargoDeps {
            inherit src;
          };
        };

        # Build dependencies only (for caching)
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Helper function to create a check with common args
        # Usage: mkCheck "nextest" "--workspace --all-targets --all-features"
        mkCheck =
          checkType: args:
          let
            checks = {
              nextest = {
                builder = craneLib.cargoNextest;
                argsAttr = "cargoNextestExtraArgs";
              };
              clippy = {
                builder = craneLib.cargoClippy;
                argsAttr = "cargoClippyExtraArgs";
              };
              test = {
                builder = craneLib.cargoTest;
                argsAttr = "cargoTestExtraArgs";
              };
              doc = {
                builder = craneLib.cargoDoc;
                argsAttr = "cargoDocExtraArgs";
              };
            };
            checkConfig = checks.${checkType};
          in
          checkConfig.builder (
            commonArgs // { inherit cargoArtifacts; } // { ${checkConfig.argsAttr} = args; }
          );

        # Automatically generate convenience wrappers for all checks
        mkCheckPackages =
          checks:
          pkgs.lib.mapAttrs' (name: value: {
            name = "check-" + name;
            value = value;
          }) checks;

        mkGitHooks =
          hooks:
          pkgs.writeShellApplication {
            name = "install-git-hooks";
            text = pkgs.lib.concatMapStrings (hookName: ''
              echo "⚡️ Installing ${hookName} hook"
              cat > "$PWD/.git/hooks/${hookName}" << 'EOF'
              ${pkgs.runtimeShell}
              set -euo pipefail
              ${hooks.${hookName}}
              EOF
              chmod +x "$PWD/.git/hooks/${hookName}"
            '') (pkgs.lib.attrNames hooks);
          };

        # Define checks that can be reused in packages
        checks = {
          formatting = treefmt.check self;

          tests = mkCheck "nextest" "--workspace --all-targets --no-default-features";
          tests-all-features = mkCheck "nextest" "--workspace --all-targets --all-features";
          clippy = mkCheck "clippy" "--workspace --all --all-targets --all-features -- --deny warnings";
          doc-tests = mkCheck "test" "--workspace --doc --all-features";
          doc = mkCheck "doc" "--workspace --no-deps --all-features";
        };
      in
      {
        # for `nix fmt`
        formatter = treefmt.wrapper;
        # for `nix flake check`
        inherit checks;

        devShells = {
          default = pkgs.mkShell {
            nativeBuildInputs = buildInputs ++ [
              rustToolchains.stable
              treefmt.wrapper
            ];
            # Nightly compiler to run miri tests
            nightly = pkgs.mkShell {
              nativeBuildInputs = [ rustToolchains.nightly ];
            };
          };
        };

        packages = {
          # Benchmarks package for local performance testing
          benchmarks = pkgs.writeShellApplication {
            name = "run-benchmarks";
            runtimeInputs = [ rustToolchains.stable ] ++ buildInputs;
            text = ''
              cargo bench --workspace --all-features
            '';
          };

          # Semver compatibility checks (requires network access to crates.io)
          check-semver = pkgs.writeShellApplication {
            name = "run-semver-checks";
            runtimeInputs =
              let
                # FIXME: Remove this override once https://github.com/NixOS/nixpkgs/issues/413204 is fixed.
                cargo-semver-checks = pkgs.cargo-semver-checks.overrideAttrs (old: {
                  doCheck = false;
                  checkPhase = "true";
                });
              in
              [
                rustToolchains.msrv
                cargo-semver-checks
              ]
              ++ buildInputs;
            text = "cargo semver-checks";
          };

          # Convenience script to install git hooks
          git-install-hooks = mkGitHooks {
            "pre-commit" = ''
              echo "⚡️ Running pre-commit checks..."
              nix build .#check-formatting -L
            '';
            "pre-push" = ''
              echo "⚡️ Running flake checks..."
              nix flake check -L
              echo "⚡️ Running semver checks..."
              nix run .#check-semver -L
            '';
          };
        }
        # Convenience wrappers to run specific checks
        // mkCheckPackages checks;
      }
    );
}
