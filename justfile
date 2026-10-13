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

# --- Generated levels (ENDLESS) ---------------------------------------------

# Bot-labelled training corpus + vocabulary (tools/levelgen/, not committed)
levelgen-corpus n="20000":
  cd {{crate}} && cargo run --release --example levelgen_vocab > ../tools/levelgen/vocab.txt
  cd {{crate}} && cargo run --release --example levelgen_corpus -- {{n}} ../tools/levelgen/corpus.txt

# Where the bots fail, per motif and profile (tune the teacher with it)
levelgen-report n="20":
  cd {{crate}} && cargo run --release --example levelgen_report -- {{n}}

# Supervised pre-training (CPU PyTorch in the training shell): sft.pt + int8 levelgen.bin
levelgen-train:
  nix develop .#training -c python3 -u {{web}}/tools/levelgen/train.py

# GRPO against the bots (Rust rollouts, PyTorch gradients) + gated int8 export
levelgen-rl:
  cd {{crate}} && cargo build --release --example levelgen_rollout --example levelgen_eval
  nix develop .#training -c python3 -u {{web}}/tools/levelgen/grpo.py

# Evaluate the embedded level model (acceptance per cell, difficulty, latency)
levelgen-eval n="20":
  cd {{crate}} && cargo run --release --example levelgen_eval -- --n {{n}}
