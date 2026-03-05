{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    fenix.url = "github:nix-community/fenix/monthly";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-utils.url = "github:numtide/flake-utils";
    rust-dev-flake.url = "github:alekseysidorov/rust-dev-flake";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
      treefmt-nix,
      rust-dev-flake,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        # Common nix packages
        pkgs = nixpkgs.legacyPackages.${system};
        # Fenix Rust toolchains
        fenixPackage = fenix.packages.${system};
        # Minimum supported Rust version
        rustVersions = {
          msrv = {
            name = "1.89.0";
            sha256 = "sha256-+9FmLhAOezBZCOziO0Qct1NOrfpjNsXxc/8I0c7BdKE=";
          };
          # Rust toolchain versions used in this project
          stable = {
            name = "1.92.0";
            sha256 = "sha256-sqSWJDUxc+zaz1nBWMAJKTAGBuGWP25GCftIOlCEAtA=";
          };
        };
        # Complete toolchains set
        rustToolchains = {
          stable = (fenixPackage.fromToolchainName rustVersions.stable).completeToolchain;
          msrv = (fenixPackage.fromToolchainName rustVersions.msrv).defaultToolchain;
          nightly = fenixPackage.complete.withComponents [ "rustfmt" ];
        };

        # Common runtime inputs used in this project.
        runtimeInputs = [
          rustToolchains.stable
        ];

        # Import rust dev flake
        rustDev = rust-dev-flake.lib.mkRustDevHelpers {
          inherit system self runtimeInputs;
          toolchain = rustToolchains.msrv;
        };

        # Eval the treefmt configuration
        treefmt = (treefmt-nix.lib.evalModule pkgs) {
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
      in
      {
        # for `nix fmt`
        formatter = treefmt.config.build.wrapper;
        # for `nix flake check`
        checks = {
          formatting = treefmt.config.build.check self;
          test = rustDev.mkCargoCheck "nextest" "--workspace --all-targets --no-default-features";
          test-all-features = rustDev.mkCargoCheck "nextest" "--workspace --all-targets --all-features";
          clippy = rustDev.mkCargoCheck "clippy" "--workspace --all-targets --all-features -- -D warnings";
          doc = rustDev.mkCargoCheck "doc" "--workspace --all-features --no-deps";
          doctest = rustDev.mkCargoCheck "test" "--doc --workspace --all-features";
        };
        # for `nix develop` and direnv
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = runtimeInputs;
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
              nix build .#check-formatting -L
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
      }
    );
}
