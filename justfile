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
    @echo "tauri:  $(./node_modules/.bin/tauri --version 2>/dev/null || echo 'not installed (run: just install)')"

# Install frontend + Rust dependencies.
[group('setup')]
install:
    pnpm install
    ./node_modules/.bin/lefthook install
    cd src-tauri && cargo fetch

# Run the app. Targets: `desktop` (Tauri shell, default), `web` (Vite only — frontend hot-reload without the Rust rebuild).
[group('dev')]
run target="desktop":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{target}}" in
        desktop) pnpm tauri dev ;;
        web)     pnpm dev ;;
        *) echo "unknown target: {{target}} (expected: desktop, web)" >&2; exit 1 ;;
    esac

# All quality gates. Checks only, never writes; `just fmt` fixes.
[group('quality')]
check:
    ./node_modules/.bin/tsc -b --noEmit
    ./node_modules/.bin/biome check .
    ./node_modules/.bin/vite build --logLevel warn
    cd src-tauri && cargo fmt --check
    cd src-tauri && cargo clippy --all-targets -- -D warnings
    cd src-tauri && cargo test

# Writes the formatters' fixes (Biome, cargo fmt).
[group('quality')]
fmt:
    ./node_modules/.bin/biome check --write .
    cd src-tauri && cargo fmt

# Production bundle (Tauri installer artifacts).
[group('build')]
build:
    pnpm tauri build
