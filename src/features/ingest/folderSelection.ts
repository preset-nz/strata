import type { FolderRow } from "./api"

export type BoxState = "checked" | "unchecked" | "mixed"

function isWithin(path: string, ancestor: string): boolean {
  return ancestor === "" || path === ancestor || path.startsWith(`${ancestor}/`)
}

/** Every folder starts checked: the common case stays one click. */
export function allFolders(rows: FolderRow[]): Set<string> {
  return new Set(rows.map((r) => r.path))
}

/**
 * A box reads checked when the folder and everything below it is kept,
 * unchecked when none of it is, mixed otherwise. The folder's own files
 * follow its own entry in `kept`.
 */
export function boxState(rows: FolderRow[], kept: Set<string>, path: string): BoxState {
  let some = false
  let all = true
  for (const r of rows) {
    if (!isWithin(r.path, path)) continue
    if (kept.has(r.path)) some = true
    else all = false
  }
  if (all) return "checked"
  return some ? "mixed" : "unchecked"
}

/**
 * Clicking a box sets the folder and its subfolders together: a checked box
 * clears the subtree, an unchecked or mixed one fills it.
 */
export function toggleFolder(rows: FolderRow[], kept: Set<string>, path: string): Set<string> {
  const on = boxState(rows, kept, path) !== "checked"
  const next = new Set(kept)
  for (const r of rows) {
    if (!isWithin(r.path, path)) continue
    if (on) next.add(r.path)
    else next.delete(r.path)
  }
  return next
}

/** "N images from M folders": only folders that contribute files count. */
export function selectionTotals(rows: FolderRow[], kept: Set<string>): { images: number; folders: number } {
  let images = 0
  let folders = 0
  for (const r of rows) {
    if (!kept.has(r.path) || r.own === 0) continue
    images += r.own
    folders += 1
  }
  return { images, folders }
}
