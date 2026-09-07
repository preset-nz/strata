// Sort axes for the catalog sheets. Lives apart from `LeftRail` so the rail
// module exports only its component (Fast Refresh) and so `features/library/api`
// can name a sort key without pulling a component into its import graph.

export const SORT_KEYS = [
  "imported",
  "filename",
  "created",
  "updated",
  "colour",
  "deleted",
] as const

export type SortKey = (typeof SORT_KEYS)[number]

export const LIBRARY_SORT_KEYS: SortKey[] = [
  "imported",
  "filename",
  "created",
  "updated",
  "colour",
]
export const TRASH_SORT_KEYS: SortKey[] = ["deleted"]

export const SORT_LABEL: Record<SortKey, string> = {
  imported: "Imported",
  filename: "Filename",
  created: "Created",
  updated: "Updated",
  colour: "Colour",
  deleted: "Deleted",
}

export type SortDirection = "asc" | "desc"

export const DEFAULT_DIRECTION: Record<SortKey, SortDirection> = {
  imported: "desc",
  filename: "asc",
  created: "desc",
  updated: "desc",
  colour: "asc",
  deleted: "desc",
}
