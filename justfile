set dotenv-load := true

# List available tasks
_default:
  @just --list

# Install JS deps
install:
  cd snes-wasm && npm install

# Run dev server
dev:
  cd snes-wasm && npm run dev

# Build WASM package (web target)
wasm-build:
  cd snes-wasm/wasm && wasm-pack build --target web --out-dir ../wasm/pkg

# Run Rust tests
wasm-test:
  cd snes-wasm/wasm && cargo test

# Build web bundle
build:
  cd snes-wasm && npm run build
