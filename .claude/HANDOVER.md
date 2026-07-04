# Handover — strata, 2026-05-24 EVE

You're picking up after a long session. Read `~/rhizomatic-preset/initiatives/strata/CLAUDE.md` for repo conventions; this brief is the state at handover.

## Where we are

**Epic 11 (soft delete + Trash) — MVP shipped end to end.** All four slices landed: schema + default filter, right-click + Undo snackbar, drag-to-trash with System rail section, Trash view + days-remaining badge + hard-delete cascade + app-start expiry sweep. Plus the surrounding follow-ups that the gesture-bridge work surfaced.

**Cross-product native gesture bridge — shipped.** Force Touch on macOS now arrives via a real `NSEvent` local monitor instead of the unreliable `webkitmouseforcedown` event. The design doc at `~/rhizomatic-preset/guidance/design/native-gesture-bridge.md` has been corrected against what we found in practice (see "Lessons from the first port"). Strata is the first consumer; the pattern is now warm for future Tauri apps.

All commits are local-only on `main`; nothing pushed.

## Commits this session (oldest → newest)

```
f37f9d0  feat(soft-delete): images.deleted_at + default-hide filter on list queries
5dd8766  feat(soft-delete): right-click Move to Trash + 5s Undo snackbar
a2e03ae  fix(db): migration v6 drops FK constraints from side tables
be5599e  feat(soft-delete): draggable cards + Trash drop target in a new System rail
c83fbef  fix(db): drop indexes explicitly before recreating tables in v6
799c24c  feat(soft-delete): hard-delete cascade + app-start expiry sweep
f3870c0  feat(trash): clickable Trash collection with days-remaining badge + restore/purge
ce04467  feat(rail): Library row + dim filters while in Trash
de0fd91  feat(trash): stronger drop feedback + collapse day-badge to first card
dcff060  fix(rail): paint Trash drop feedback in WebKit
6ccd932  fix(dnd): disable Tauri interceptor so internal HTML5 DnD works
90488fe  fix(hard-press): cancel long-press timer on dragstart
16a050b  feat(gestures): native Force Touch bridge → force-touch Tauri event
3dabfbc  fix(gestures): contain panics across the ObjC FFI boundary
0bb8056  fix(gestures): monitor NSEventMaskPressure, not mouse-down
483cc62  fix(gestures): flip Y from Cocoa bottom-left to CSS top-left
bf362ac  fix(library): select card on force-touch activate, not just on release
1bf7d9f  fix(nav): rail clicks always leave the in-progress ingest view
```

Documentation updates landed in `~/rhizomatic-preset/guidance/` (not committed there — Georg bundles guidance edits himself):
- `design/native-gesture-bridge.md` — corrected mask, added Y-flip section, added "Lessons from the first port" with the three corrections.
- `projects/strata/features/13-library-shell/quickview-on-hard-press.md` — long-press dropped; Spacebar promoted to canonical non-haptic path.
- `projects/strata/features/11-soft-delete-retention/README.md` — slices marked shipped with commit refs; "Follow-ups" section replaces "Open questions"; "smart-collections" rail re-architecture flagged.
- `projects/strata/features/roadmap.md` — Active-now line updated; epic 11 marked MVP shipped.

## Architecture notes that aren't obvious from the code

### Why Tauri's native drag-drop interceptor is off (`dragDropEnabled: false`)

WebKit (Tauri's macOS engine) hides `dataTransfer.types` during `dragenter`/`dragover` for security — payload validation can't gate the visual hover state. Even without that, Tauri's native interceptor at the OS level eats drag events before they reach the webview, so internal HTML5 DnD (card → Trash row) never received the events the JS handlers were waiting for. We flipped the flag in `tauri.conf.json` and rewrote `DropZone` to use window-level HTML5 `drop` listeners reading `e.dataTransfer.files[0].path` (wry-exposed non-standard property on macOS).

Knock-on: if you need OS-to-Web file metadata beyond the path, the Tauri `@tauri-apps/api/webview` `onDragDropEvent` API no longer fires — use HTML5's File API.

### Why DuckDB side tables don't have FK constraints anymore

DuckDB refuses `UPDATE` on a parent row that has FK-referenced children, even for non-key columns. That broke `delete_image`'s `UPDATE images SET deleted_at = now()`. Migration v6 recreated the four side tables (`image_palette`, `image_palette_bucket`, `image_metadata`, `image_keyword`) without `REFERENCES images(id)`. The hard-delete cascade is managed explicitly in Rust (`purge_one`), so FK enforcement wasn't carrying weight.

V6 also drops indexes explicitly before recreating tables — leaving the implicit cleanup to DuckDB triggered an `unbound_index.cpp` assertion at startup. Any DB that went through the broken v6 had to be nuked; the user's was, and the v6 migration was made idempotent against fresh DBs.

### Why the Trash view is event-driven, not callback-driven

Slice 3c needed both right-click and drag-drop gestures to end at the same place with the same Undo affordance. Hoisting `moveToTrash` state to App and passing refs around got ugly. Cleaner: the backend `delete_image` / `restore_image` / `purge_image` commands emit Tauri events (`library://images-trashed`, `library://images-restored`, `library://images-purged`) after the SQL succeeds. LibrarySheet and TrashSheet listen and update their own grid state from the events. The hook `useMoveToTrash` is a thin orchestration shim — API call + snackbar — that any caller can use without knowing about UI state.

### Why long-press isn't a Quickview fallback

Initial plan was `useHardPress = useForceTouch + long-press`. The 350ms press timer collided with HTML5 drag: the browser suppresses `pointermove` during a drag session, so the motion-threshold cancel couldn't fire and Quickview popped mid-drag-to-trash. Dropped at user direction. Non-haptic users will reach Quickview via Spacebar (when focus + keyboard nav lands) or via a context-menu entry.

## Open follow-ups (most → least pressing)

1. **Bulk-select delete with confirm-at-≥50.** The selection store is single-image (`Selection = none | image | batch`). Needs multi-select (probably in epic 08) before this can land.
2. **Reopen the JobProgress view after navigating away.** Currently the rail click is the universal escape hatch from any in-progress ingest view; backend continues but there's no UI to return to it. Add a header indicator + click-to-resume.
3. **Dedup-on-reimport restores the soft-deleted row.** Backend dedup in `ingest/job.rs` finds soft-deleted rows (UNIQUE on content_hash) and currently reports `skipped-duplicate`. Should clear `deleted_at` and report `restored`.
4. **Smart-collections rail re-architecture.** Georg's call: buckets / labels / batches are more naturally "smart collections" peer to Library and Trash than as filters. Current dim-when-inactive in Trash mode is a stopgap. Future epic-13-shaped item.
5. **Daily timer for hard-delete sweep.** App-start sweep covers MVP. Add recurring timer if always-on sessions accumulate stale rows.
6. **Retention setting.** Hardcoded 30 days; setting hook lands when epic 12 (Settings) does. Retention is a property of "now", not "when deleted" — drops apply retroactively.
7. **Keyboard Quickview (Spacebar).** Needed before non-haptic users have any Quickview affordance. Depends on focus management / keyboard navigation in cards.
8. **"Rehydrate catalog from store" recipe.** User noted that `store/` is the safety net — a Tauri command that walks the content-addressed store and recreates `images` rows from binary hashes would unblock disaster recovery without needing original source folders.

## Key files and paths

### Backend (Rust)

- `src-tauri/src/lib.rs` — Tauri commands, sort/filter SQL, delete/restore/purge cascade, app-start expiry sweep.
- `src-tauri/src/gestures/` — macOS-gated gesture monitors. Today: `force_touch.rs`.
- `src-tauri/src/db/mod.rs` — migrations through v6.
- `src-tauri/tauri.conf.json` — `app.windows[0].dragDropEnabled: false`.

### Frontend (TS)

- `src/App.tsx` — AppShell, panel state, rail-as-escape-hatch wiring.
- `src/components/shell/LeftRail.tsx` — Library nav row, Sort with per-panel options, dim-when-inactive filters, Trash drop target.
- `src/components/image-card/` — `ImageCard`, `CardContextMenu` (mode-aware library/trash), `use-draggable-card.ts` primitive.
- `src/components/ui/snackbar.tsx` — base-ui Toast wrapper. Provider at App root; Viewport rendered per panel for grid-anchored positioning.
- `src/features/library/api.ts` — `listImages` with `includeDeleted`/`onlyDeleted`; `deleteImages` / `restoreImages` / `purgeImages`; `libraryCount({ onlyDeleted })`.
- `src/features/library/use-move-to-trash.ts` — shared orchestration hook (API + snackbar).
- `src/features/library/LibrarySheet.tsx` — listens to `library://images-trashed` / `images-restored` for grid sync.
- `src/features/trash/TrashSheet.tsx` — `only_deleted: true`, day-grouped badge, Restore / Delete permanently context menu.
- `src/lib/gestures/use-force-touch.ts` — typed wrapper, Y-flip on the TS side.
- `src/lib/use-hard-press.ts` — per-card hit-tester over `useForceTouch`.

### Tauri events emitted

- `force-touch` — from the NSEvent monitor; payload `{ pressure, x, y }` (Y in Cocoa coords until the TS hook flips).
- `library://images-trashed` — payload `string[]` of trashed ids.
- `library://images-restored` — payload `string[]` of restored ids.
- `library://images-purged` — payload `string[]` of hard-deleted ids.

### Persistence keys (localStorage)

- `strata.rail.collapsed` — `Record<SectionId, boolean>`.
- `strata.library.sort` — `{ key: SortKey, direction: SortDirection }`.
- `strata.trash.sortDirection` — `SortDirection` (sort key is implicit `"deleted"`).

## Conventions to honour

- NZ English in user-facing strings (`colour`, `behaviour`); identifiers stay standard.
- No emojis in code/docs.
- Dates: YYYY-MM-DD absolute.
- Commit autonomously per logical unit; **never push** without explicit ask.
- For UI changes: commit, ask Georg to `just run web` or `just run desktop` to verify, then proceed. The "Visual verification pending" pattern is established.
- Guidance docs (`~/rhizomatic-preset/guidance/`) — edits stay uncommitted unless Georg explicitly asks. He bundles them himself.

## Start of next session

Reasonable opening move: ask Georg what the next concern is. Likely candidates are 1, 2, or 3 from the follow-ups list, or a fresh direction.
