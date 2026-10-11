{
  description = "SNES-like platformer (Rust/WASM + Web) dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            just
            nodejs_24
            cargo
            rustc
            rustfmt
            clippy
            wasm-pack
            lld
          ];

          shellHook = ''
            echo "Dev shell ready. Try: just --list"
          '';
        };
      });
}
