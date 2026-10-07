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

# All quality gates: Rust check + tests, TypeScript type-check, lint.
# eslint fails on errors only; the one standing warning is TanStack Virtual's
# `incompatible-library`, which no local change can clear.
[group('quality')]
check:
    cd src-tauri && cargo check
    cd src-tauri && cargo test
    ./node_modules/.bin/tsc -b --noEmit
    ./node_modules/.bin/eslint .

# Production bundle (Tauri installer artifacts).
[group('build')]
build:
    pnpm tauri build
