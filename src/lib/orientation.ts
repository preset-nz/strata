// Orientation buckets, mirroring `src-tauri/src/ingest/orientation.rs`.
// Order is wider -> equal -> taller, the same order the sort key uses.

export const ORIENTATIONS = ["landscape", "square", "portrait"] as const

export type Orientation = (typeof ORIENTATIONS)[number]

export const ORIENTATION_LABEL: Record<Orientation, string> = {
  landscape: "Landscape",
  square: "Square",
  portrait: "Portrait",
}
