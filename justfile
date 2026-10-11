set dotenv-load := true

web := "snes-wasm"
crate := "snes-wasm/wasm"

# List available tasks
_default:
  @just --list

# Install JS deps (only if missing)
install:
  cd {{web}} && [ -d node_modules ] || npm ci

# Regenerate the original art (stdlib-only Python; audio is synthesized in the wasm)
assets:
  cd {{web}} && python3 tools/gen_art.py

# Regenerate public/levels/*.json (overwrites edits made in Tiled!)
level:
  cd {{web}} && python3 tools/gen_level.py

# Build the WASM package (web target) into snes-wasm/wasm/pkg
wasm-build:
  cd {{crate}} && wasm-pack build --release --target web --out-dir pkg

# Run Rust tests (physics, rules, Tiled parser, bot playthrough of levels 1-4;
# the wasm crate carries the music engine)
wasm-test:
  cd {{crate}} && cargo test

# Run the dev server (http://localhost:5173)
dev: install wasm-build
  cd {{web}} && npm run dev

# Production build into snes-wasm/dist
build: install wasm-build
  cd {{web}} && npm run build

# Build then serve the production bundle (http://localhost:4173)
serve: build
  cd {{web}} && npm run preview

# Headless-browser smoke test against the production build
e2e: build
  cd {{web}} && npm run e2e

# Format Rust code
fmt:
  cd {{crate}} && cargo fmt

# Lint: rustfmt check, clippy (warnings = errors), TypeScript typecheck
lint: install wasm-build
  cd {{crate}} && cargo fmt --check && cargo clippy --all-targets -- -D warnings
  cd {{web}} && npm run typecheck

# Everything: lint, unit tests, build, browser smoke test
check: lint wasm-test e2e

# Alias kept from the original plan
test: wasm-test
