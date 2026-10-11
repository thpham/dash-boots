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
            # Rust → WASM
            cargo
            rustc
            rustfmt
            clippy
            lld
            wasm-pack
            # Must match the `wasm-bindgen` version pinned in snes-wasm/wasm/Cargo.toml
            # so wasm-pack uses it instead of downloading one.
            wasm-bindgen-cli
            binaryen # wasm-opt
            # Asset generators (stdlib only)
            python3
          ] ++ lib.optionals stdenv.hostPlatform.isLinux [
            # Headless browser for `just e2e`
            chromium
          ];

          shellHook = ''
            echo "Dev shell ready. Try: just --list"
          '';
        };
      });
}
