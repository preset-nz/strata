# Strata — Tauri + Vite + React + Rust local-first image catalog.
# Standard verbs: prep, install, run, check, build.

# Show all recipes.
default:
    @just --list

# Print versions of required host tools. Flags anything MISSING.
[group('setup')]
prep:
    @echo "node:   $(node --version 2>/dev/null || echo MISSING)"
    @echo "npm:    $(npm --version 2>/dev/null || echo MISSING)"
    @echo "rustc:  $(rustc --version 2>/dev/null || echo MISSING)"
    @echo "cargo:  $(cargo --version 2>/dev/null || echo MISSING)"
    @echo "tauri:  $(npx --no-install tauri --version 2>/dev/null || echo 'not installed (run: just install)')"

# Install frontend + Rust dependencies.
[group('setup')]
install:
    npm install
    cd src-tauri && cargo fetch

# Run the app. Targets: `desktop` (Tauri shell, default), `web` (Vite only — frontend hot-reload without the Rust rebuild).
[group('dev')]
run target="desktop":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{target}}" in
        desktop) npm run tauri dev ;;
        web)     npm run dev ;;
        *) echo "unknown target: {{target}} (expected: desktop, web)" >&2; exit 1 ;;
    esac

# All quality gates: Rust check + tests, TypeScript type-check, lint.
# eslint fails on errors only; the one standing warning is TanStack Virtual's
# `incompatible-library`, which no local change can clear.
[group('quality')]
check:
    cd src-tauri && cargo check
    cd src-tauri && cargo test
    npx tsc -b --noEmit
    npx eslint .

# Production bundle (Tauri installer artifacts).
[group('build')]
build:
    npm run tauri build
