{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , rust-overlay
    , treefmt-nix
    }: flake-utils.lib.eachDefaultSystem (system:
    let
      # Minimum supported Rust version
      msrv = "1.78.0";
      # Setup nixpkgs
      pkgs = import nixpkgs {
        inherit system;

        overlays = [
          rust-overlay.overlays.default
          (final: prev: {
            rustToolchains = {
              msrv = prev.rust-bin.stable.${msrv}.default;
              stable = prev.rust-bin.stable.latest.default.override {
                extensions = [
                  "rust-src"
                  "rust-analyzer"
                ];
              };
              nightly = prev.rust-bin.nightly.latest.default;
            };
          })
        ];
      };
      # Setup runtime dependencies
      runtimeInputs = with pkgs; [
        cargo-nextest
        openssl
        pkg-config
      ]
      # Some additional libraries for the Darwin platform
      ++ lib.optionals stdenv.isDarwin [
        darwin.apple_sdk.frameworks.SystemConfiguration
      ];

      # Eval the treefmt modules from ./treefmt.nix
      treefmt = (treefmt-nix.lib.evalModule pkgs ./treefmt.nix).config.build;
      # CI scripts
      ci = with pkgs; {
        tests = writeShellApplication {
          name = "ci-run-tests";
          runtimeInputs = with pkgs; [ rustToolchains.msrv ] ++ runtimeInputs;
          text = ''
            cargo nextest run --workspace --all-targets --no-default-features
            cargo nextest run --workspace --all-targets --all-features

            cargo test --workspace --doc --no-default-features
            cargo test --workspace --doc --all-features

            cargo run --example rate_limiter
            cargo run --example retry
          '';
        };

        lints = writeShellApplication {
          name = "ci-run-lints";
          runtimeInputs = with pkgs; [ rustToolchains.stable typos ] ++ runtimeInputs;
          text = ''
            typos
            cargo clippy --workspace --all --no-default-features
            cargo clippy --workspace --all --all-targets --all-features
            cargo doc --workspace --no-deps --no-default-features
            cargo doc --workspace --no-deps --all-features
          '';
        };

        semver_checks = writeShellApplication {
          name = "ci-run-semver-checks";
          runtimeInputs = with pkgs; [
            rustToolchains.msrv
            cargo-semver-checks
          ] ++ runtimeInputs;
          text = ''cargo semver-checks'';
        };

        # Run them all together
        all = writeShellApplication {
          name = "ci-run-all";
          runtimeInputs = [ ci.lints ci.tests ];
          text = ''
            ci-run-lints
            ci-run-tests
            ci-run-semver-checks
          '';
        };
      };

      mkCommand = shell: command:
        pkgs.writeShellApplication {
          name = "cmd-${shell}-${command}";
          runtimeInputs = [ pkgs.nix ];
          text = ''nix develop ".#${shell}" --command "${command}"'';
        };

      mkCommandDefault = mkCommand "default";
    in
    {
      # for `nix fmt`
      formatter = treefmt.wrapper;
      # for `nix flake check`
      checks.formatting = treefmt.check self;

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; runtimeInputs ++ [
          rustToolchains.stable
          ci.all
          ci.lints
          ci.tests
          ci.semver_checks
        ];
      };

      # Nightly compilator to run miri tests
      devShells.nightly = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          rustToolchains.nightly
        ];
      };

      packages = {
        ci-lints = mkCommandDefault "ci-run-lints";
        ci-tests = mkCommandDefault "ci-run-tests";
        ci-semver-checks = mkCommandDefault "ci-run-semver-checks";
        ci-all = mkCommandDefault "ci-run-all";
        git-install-hooks = pkgs.writeShellScriptBin "install-git-hook"
          ''
            echo "-> Installing pre-commit hook"
            echo "nix flake check" >> "$PWD/.git/hooks/pre-commit"
            chmod +x "$PWD/.git/hooks/pre-commit"

            echo "-> Installing pre-push hook"
            echo "nix run \".#ci-all\"" >> "$PWD/.git/hooks/pre-push"
            chmod +x "$PWD/.git/hooks/pre-push"
          '';
      };
    });
}
