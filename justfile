# Strata — Tauri + Vite + React + Rust local-first image catalog.
# Standard verbs: prep, install, run, check, build.

# Show all recipes.
default:
    @just --list

# Print versions of required host tools. Flags anything MISSING.
[group('setup')]
prep:
    @echo "node:   $(node --version 2>/dev/null || echo MISSING)"
    @echo "pnpm:   $(pnpm --version 2>/dev/null || echo MISSING)"
    @echo "rustc:  $(rustc --version 2>/dev/null || echo MISSING)"
    @echo "cargo:  $(cargo --version 2>/dev/null || echo MISSING)"
    @echo "preset-compliance: $(preset-compliance --version 2>/dev/null || echo 'MISSING (cargo binstall preset-compliance)')"
    @echo "tauri:  $(./node_modules/.bin/tauri --version 2>/dev/null || echo 'not installed (run: just install)')"

# Install frontend + Rust dependencies.
[group('setup')]
install:
    pnpm install
    ./node_modules/.bin/lefthook install
    cd src-tauri && cargo fetch

# Run the app in its Tauri window. There is no browser-only mode (family rule, 2026-10-08).
[group('dev')]
run:
    pnpm tauri dev

# All quality gates. Checks only, never writes; `just fmt` fixes.
[group('quality')]
check:
    ./node_modules/.bin/tsc -b --noEmit
    ./node_modules/.bin/biome check .
    ./node_modules/.bin/vite build --logLevel warn
    cd src-tauri && cargo fmt --check
    cd src-tauri && cargo clippy --all-targets -- -D warnings
    cd src-tauri && cargo test
    just licences

# Writes the formatters' fixes (Biome, cargo fmt).
[group('quality')]
fmt:
    ./node_modules/.bin/biome check --write .
    cd src-tauri && cargo fmt

# Production bundle (Tauri installer artifacts).
[group('build')]
build:
    pnpm tauri build

# Licence check against the committed lock. Reads files only, no network.
# Re-resolve with `preset-compliance licences scan` after changing dependencies.
[group('quality')]
licences:
    preset-compliance licences check

# Version, changelog, commit and tag from the conventional commits since the last
# tag (knope.toml). Pushing stays by hand.
[group('build')]
release:
    knope release

# What `release` would do, without touching anything.
[group('build')]
release-preview:
    knope release --dry-run
