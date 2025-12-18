{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    fenix.url = "github:nix-community/fenix/monthly";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
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

        # CI scripts
        ci = {
          tests = pkgs.writeShellApplication {
            name = "ci-run-tests";
            runtimeInputs = [ rustToolchains.msrv ] ++ runtimeInputs;
            text = ''
              cargo nextest run --workspace --all-targets --no-default-features
              cargo nextest run --workspace --all-targets --all-features

              cargo test --workspace --doc --no-default-features
              cargo test --workspace --doc --all-features

              cargo run --example rate_limiter
              cargo run --example retry
            '';
          };

          lints = pkgs.writeShellApplication {
            name = "ci-run-lints";
            runtimeInputs = [
              rustToolchains.stable
              pkgs.typos
            ]
            ++ runtimeInputs;
            text = ''
              typos
              cargo clippy --workspace --all --no-default-features
              cargo clippy --workspace --all --all-targets --all-features
              cargo doc --workspace --no-deps --no-default-features
              cargo doc --workspace --no-deps --all-features
            '';
          };

          benchmarks = pkgs.writeShellApplication {
            name = "ci-run-benchmarks";
            runtimeInputs = [ rustToolchains.stable ] ++ runtimeInputs;
            text = ''
              cargo bench --workspace --all-features
            '';
          };

          semver_checks = pkgs.writeShellApplication {
            name = "ci-run-semver-checks";
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
              ++ runtimeInputs;
            text = ''cargo semver-checks'';
          };

          # Run them all together
          all = pkgs.writeShellApplication {
            name = "ci-run-all";
            runtimeInputs = [
              ci.lints
              ci.tests
              ci.semver_checks
            ];
            text = ''
              ci-run-lints
              ci-run-tests
              ci-run-semver-checks
            '';
          };
        };
      in
      {
        # for `nix fmt`
        formatter = treefmt.wrapper;
        # for `nix flake check`
        checks.formatting = treefmt.check self;

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = runtimeInputs ++ [
            rustToolchains.stable
            treefmt.wrapper
            ci.all
          ];
        };

        # Nightly compiler to run miri tests
        devShells.nightly = pkgs.mkShell {
          nativeBuildInputs = [ rustToolchains.nightly ];
        };

        packages = {
          ci-all = ci.all;
          ci-lints = ci.lints;
          ci-tests = ci.tests;
          ci-semver-checks = ci.semver_checks;
          ci-benchmarks = ci.benchmarks;

          git-install-hooks = pkgs.writeShellApplication {
            name = "install-git-hooks";
            text = ''
              echo "-> Installing pre-commit hook"
              echo "nix flake check" >> "$PWD/.git/hooks/pre-commit"
              chmod +x "$PWD/.git/hooks/pre-commit"

              echo "-> Installing pre-push hook"
              echo "nix run \".#ci-all\"" >> "$PWD/.git/hooks/pre-push"
              chmod +x "$PWD/.git/hooks/pre-push"
            '';
          };
        };
      }
    );
}
