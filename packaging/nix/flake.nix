{
  description = "Aporia — local-first notebook for philosophical thinking";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        aporia = pkgs.callPackage ./package.nix { };
      in
      {
        packages.default = aporia;
        packages.aporia = aporia;

        devShells.default = pkgs.mkShell {
          inputsFrom = [ aporia ];
          packages = with pkgs; [
            cargo
            clippy
            rustc
            rustfmt
            cargo-deny
            cargo-audit
            gitleaks
            typos
            ast-grep
            markdownlint-cli
            shellcheck
            actionlint
          ];
        };

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
