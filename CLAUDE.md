# CLAUDE.md — strata (code repo)

Tauri + Vite + React frontend, Rust orchestrator. Local-first image catalog with derived visual intelligence (embeddings, palettes, segmentation).

## Where things live

- **Design / planning docs:** [`~/rhizomatic-preset/guidance/projects/strata/`](../../guidance/projects/strata/) — product brief, tech stack, feature roadmap, epic READMEs. Read this first to understand what we're building and why.
- **Source spec (immutable reference):** `~/Documents/obsidian.md/rdg/rdg/The List/Strata - Image Archeology/` — 21 .md notes capturing the originating thinking. Treat as the upstream source-of-truth; cite, don't restate.
- **This repo:** the running code. Comments here explain shipped code to future readers — not planning, not roadmap. Planning artefacts belong in the guidance repo.

## Architect's working principles

These shape every design call. Reference them when proposing structure, naming components, or accepting/rejecting an approach.

- **SOLID** — single responsibility per module/plugin; orchestrator open for extension (new plugins) closed for modification; storage via interfaces (adapter pattern), not concrete I/O.
- **CUPID** — composable plugins, Unix-philosophy single-purpose tools, predictable artifact contracts, idiomatic Rust/TS, domain-based naming (palette, segmentation, favourites — not `manager`, `helper`, `util`).
- **Platform thinking** — Strata is a platform for image intelligence, not a closed app. The plugin/artifact system is the platform substrate; the UI is one of many consumers. Design extension points before features.
- **IBM Design Thinking** — Hills (user outcomes, not feature lists), Sponsor Users, Playbacks, the Observe/Reflect/Make loop. Feature stories are framed as outcomes ("an architect can pull up every image with a green-mould palette in under 3 seconds"), not tasks.

## Stack at a glance

- **Shell:** Tauri 2 (desktop) — Rust backend invoked from a React/TS frontend.
- **Frontend:** Vite 7 + React 19 + TypeScript 5.8.
- **Rust:** orchestrator-to-be; today only the Tauri scaffold. Planned crates: `tokio`, `rayon`, `duckdb`, `pyo3` (later), `image`.
- **ML compute:** Python sidecar(s) for CLIP, SAM, Cellpose, optional BLIP. Subprocess dispatch (MVP) → PyO3 (later).
- **Storage:** content-addressed filesystem for binaries; DuckDB for the feature store (embeddings, palettes, artifacts, runs).

See [`guidance/projects/strata/tech-stack.md`](../../guidance/projects/strata/tech-stack.md) for the full picture.

## Conventions

- **NZ English** in user-facing strings: `colour`, `organisation`, `behaviour`. Identifiers / CSS / platform APIs stay standard.
- **No emojis** in code or docs.
- **Dates: YYYY-MM-DD.** Absolute always.
- **Frontmatter** on every `.md` in the guidance repo (see its CLAUDE.md). This repo's markdown is for shipped-code explainers; frontmatter optional.
- **Don't auto-commit** — propose, then wait.

## Tooling

Use `just <recipe>` rather than calling `pnpm` / `cargo` directly. Run `just` (no args) for the recipe list.

Standard verbs (present in every preset.nz project): `prep`, `install`, `run [target]`, `check`, `build`. Convention spec: [`guidance/runbooks/justfile-conventions.md`](../../guidance/runbooks/justfile-conventions.md). Strata's `run` takes a target — `desktop` (default, Tauri shell) or `web` (Vite-only, frontend hot-reload without rebuilding Rust).

Annotate any new recipes with `[group('setup'|'dev'|'quality'|'build'|'eval')]`.

## When in doubt

1. Read the relevant epic README in [`guidance/projects/strata/features/`](../../guidance/projects/strata/features/).
2. If still ambiguous, read the originating Obsidian note (paths cited in the epic README).
3. If still ambiguous, ask. The architect (Georg) prefers a clarifying question to a wrong half-hour.
