{
  description = "lean4losophy — local-first notebook for philosophical thinking";

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
        lean4losophy = pkgs.callPackage ./package.nix { };
      in
      {
        packages.default = lean4losophy;
        packages.lean4losophy = lean4losophy;

        devShells.default = pkgs.mkShell {
          inputsFrom = [ lean4losophy ];
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
