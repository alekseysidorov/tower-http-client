{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    fenix.url = "github:nix-community/fenix/monthly";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-dev-flake.url = "github:alekseysidorov/rust-dev-flake";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-parts,
      fenix,
      rust-dev-flake,
      treefmt-nix,
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # Declared systems that your flake supports. These will be enumerated in perSystem
      systems = nixpkgs.lib.systems.flakeExposed;

      imports = [
        treefmt-nix.flakeModule
      ];

      perSystem =
        { system, ... }:
        let
          # Common nix packages
          pkgs = nixpkgs.legacyPackages.${system};
          # Fenix Rust toolchains
          fenixPackage = fenix.packages.${system};
          # Minimum supported Rust version
          rustVersions = {
            msrv = {
              name = "1.92.0";
              sha256 = "sha256-sqSWJDUxc+zaz1nBWMAJKTAGBuGWP25GCftIOlCEAtA=";
            };
            # Rust toolchain versions used in this project
            stable = {
              name = "1.97.1";
              sha256 = "sha256-A1abGIbOtcBSdrUMhDGrER3pRM1hQP4fp9gh3Y4PKc8=";
            };
          };
          # Complete toolchains set
          rustToolchains = {
            stable = (fenixPackage.fromToolchainName rustVersions.stable).completeToolchain;
            msrv = (fenixPackage.fromToolchainName rustVersions.msrv).defaultToolchain;
            nightly = fenixPackage.complete.withComponents [ "rustfmt" ];
          };

          # Import rust dev flake
          rustDev = rust-dev-flake.lib.mkRustDevHelpers {
            inherit system self;
            # Common runtime inputs used in this project.
            runtimeInputs = [
              rustToolchains.stable
            ];
            toolchain = rustToolchains.msrv;
          };
        in
        {
          # Use the modified pkgs with overlays for all modules and packages in this system.
          _module.args = pkgs;
          # Setup nix formatting with treefmt-nix.
          treefmt = {
            # Project root marker used by treefmt
            projectRootFile = "flake.nix";

            programs = {
              nixfmt.enable = true;
              rustfmt = {
                enable = true;
                # Use nightly Rust toolchain to get more configuration options.
                package = rustToolchains.nightly;
              };
              beautysh.enable = true;
              deno.enable = true;
              taplo.enable = true;
            };
          };
          # for `nix flake check`
          checks = {
            test = rustDev.mkCargoCheck "nextest" "--workspace --all-targets --no-default-features";
            test-all-features = rustDev.mkCargoCheck "nextest" "--workspace --all-targets --all-features";
            clippy = rustDev.mkCargoCheck "clippy" "--workspace --all-targets --all-features -- -D warnings";
            doc = rustDev.mkCargoCheck "doc" "--workspace --all-features --no-deps";
            doctest = rustDev.mkCargoCheck "test" "--doc --workspace --all-features";
            audit = rustDev.mkCargoCheck "audit" "";
          };

          devShells = {
            # for `nix develop` and direnv
            default = pkgs.mkShell {
              nativeBuildInputs = [
                rustToolchains.stable

                pkgs.marksman
                pkgs.typos-lsp
                pkgs.crates-lsp
                pkgs.yaml-language-server
                pkgs.nil
                pkgs.nixd
                pkgs.tombi
              ];
            };
            # for nightly rust
            nightly = pkgs.mkShell {
              nativeBuildInputs = [
                rustToolchains.nightly
              ];
            };
          };

          # for `nix run`
          packages = (rustDev.mkCheckPackages self.checks.${system}) // {
            inherit (rustDev.runtimeChecks)
              check-cargo-semver
              check-cargo-publish
              ;

            git-install-hooks = rustDev.mkGitHooks {
              "pre-commit" = ''
                echo "⚡️ Running pre-commit checks..."
                nix build .#check-treefmt -L
              '';

              "pre-push" = ''
                echo "⚡️ Running flake checks..."
                nix flake check -L
                echo "⚡️ Running semver checks..."
                nix run .#check-cargo-semver -L
                echo "⚡️ Running cargo publish compatibility checks..."
                nix run .#check-cargo-publish -L
              '';
            };
          };
        };
    };
}
