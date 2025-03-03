# treefmt.nix
{ pkgs, ... }:
{
  # Used to find the project root
  projectRootFile = "flake.nix";

  programs.nixpkgs-fmt.enable = true;
  programs.rustfmt = {
    enable = true;
    # Fix issue "package does not have the meta.mainProgram attribute".
    package = pkgs.rustToolchains.nightly // {
      meta = pkgs.rustToolchains.nightly.meta // {
        mainProgram = "rustfmt";
      };
    };
  };
  programs.beautysh.enable = true;
  programs.deno.enable = true;
  programs.taplo.enable = true;
}
