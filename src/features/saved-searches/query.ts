import { LIBRARY_SORT_KEYS, type SortDirection, type SortKey } from "@/components/shell/sort-keys"
import { ORIENTATIONS, ORIENTATION_LABEL, type Orientation } from "@/lib/orientation"
import { VGA16_BUCKETS, VGA16_LABEL, type Vga16Bucket } from "@/lib/vga16"

/**
 * What a saved search remembers: the library query, not its results. Stored
 * as JSON the catalog never reads, so `v` lets the shape grow; a version this
 * build doesn't know is shown but can't be run.
 */
export type SavedQuery = {
  v: 1
  text: string
  buckets: Vga16Bucket[]
  orientations: Orientation[]
  batchId: string | null
  sort: SortKey
  direction: SortDirection
}

export function makeQuery(q: Omit<SavedQuery, "v">): SavedQuery {
  return {
    v: 1,
    text: q.text.trim(),
    buckets: [...q.buckets].sort(),
    orientations: [...q.orientations].sort(),
    batchId: q.batchId,
    sort: q.sort,
    direction: q.direction,
  }
}

/** Something worth saving: a typed query or at least one filter. */
export function isSavable(q: SavedQuery): boolean {
  return q.text !== "" || q.buckets.length > 0 || q.orientations.length > 0 || q.batchId !== null
}

function strings(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((x): x is string => typeof x === "string") : []
}

/** Reads a stored query, dropping values this build doesn't know. */
export function parseQuery(raw: unknown): SavedQuery | null {
  if (typeof raw !== "object" || raw === null) return null
  const r = raw as Record<string, unknown>
  if (r.v !== 1) return null
  const sort = LIBRARY_SORT_KEYS.find((k) => k === r.sort) ?? "imported"
  return makeQuery({
    text: typeof r.text === "string" ? r.text : "",
    buckets: strings(r.buckets).filter((b): b is Vga16Bucket =>
      (VGA16_BUCKETS as readonly string[]).includes(b),
    ),
    orientations: strings(r.orientations).filter((o): o is Orientation =>
      (ORIENTATIONS as readonly string[]).includes(o),
    ),
    batchId: typeof r.batchId === "string" ? r.batchId : null,
    sort,
    direction: r.direction === "asc" ? "asc" : "desc",
  })
}

export function sameQuery(a: SavedQuery, b: SavedQuery): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

/** A starting name for the save field: the words, then the filters. */
export function suggestName(q: SavedQuery): string {
  const parts = [
    q.text,
    ...q.buckets.map((b) => VGA16_LABEL[b]),
    ...q.orientations.map((o) => ORIENTATION_LABEL[o]),
    q.batchId ? "one import" : "",
  ].filter(Boolean)
  return parts.join(", ")
}
