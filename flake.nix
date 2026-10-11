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
            # The `cc` crate (libxm's C sources) needs clang to emit wasm32
            # objects; without it the fallback gcc produces host-arch .o and
            # the wasm link fails with "undefined symbol: xm_*". Use the
            # UNwrapped clang: the wrapped one injects host-only hardening
            # flags (-fzero-call-used-regs=used-gpr) and glibc includes that
            # wasm32 rejects. libxm needs no libc beyond our freestanding
            # stubs (wasm-c-stubs/), and malloc/free/memcpy resolve against
            # Rust's dlmalloc/compiler_builtins at link time.
            pkgs.llvmPackages.clang-unwrapped
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
            # libxm is the one C dependency here and it must compile for BOTH
            # the native tests and wasm32. The user's session may export
            # CC=gcc (→ host-arch .o and a broken wasm link) or the wrapped
            # clang (→ hardening flags wasm32 rejects). Force the unwrapped
            # clang, with the repo's freestanding libc stub headers on the
            # include path for every target (plain CFLAGS applies to all
            # cc-crate compilations; symbols resolve per-target at link).
            export CC="${pkgs.llvmPackages.clang-unwrapped}/bin/clang"
            export CFLAGS="''${CFLAGS:+$CFLAGS }-I${self}/wasm-c-stubs"
            echo "CC = unwrapped clang, CFLAGS += -I wasm-c-stubs (wasm32 C deps)"
          '';
        };
      });
}
