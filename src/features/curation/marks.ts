import { invoke } from "@tauri-apps/api/core"
import { create } from "zustand"

import type { ColourLabel } from "@/components/image-card/colour-label"

/** A heart and a colour label, per image across the catalog (epic 08). */
export type Mark = { favourite: boolean; label: ColourLabel | null }
export type Marked = Mark & { id: string }

const NONE: Mark = { favourite: false, label: null }

interface MarksStore {
  marks: Record<string, Mark>
  /** Take what a list query or a command returned. */
  put: (rows: Marked[]) => void
}

/**
 * One store for every grid and the menu, so a heart set in the library shows
 * in the contact sheet and checks Image › Favourite at once.
 */
export const useMarks = create<MarksStore>()((set) => ({
  marks: {},
  put: (rows) =>
    set((s) => {
      const marks = { ...s.marks }
      for (const r of rows)
        marks[r.id] = { favourite: r.favourite, label: r.label }
      return { marks }
    }),
}))

export function useMark(id: string | null): Mark {
  return useMarks((s) => (id ? (s.marks[id] ?? NONE) : NONE))
}

/** Hearts or un-hearts; one undo step. Resolves with the images that changed. */
export async function setFavourite(
  ids: string[],
  on: boolean
): Promise<Marked[]> {
  const changed = await invoke<Marked[]>("set_favourite", { ids, on })
  useMarks.getState().put(changed)
  return changed
}

/** Sets or clears the colour label; one undo step. */
export async function setColourLabel(
  ids: string[],
  label: ColourLabel | null
): Promise<Marked[]> {
  const changed = await invoke<Marked[]>("set_colour_label", { ids, label })
  useMarks.getState().put(changed)
  return changed
}

/** Loads marks for cards that didn't come from a list query. */
export async function loadMarks(ids: string[]): Promise<void> {
  if (ids.length === 0) return
  useMarks.getState().put(await invoke<Marked[]>("image_marks", { ids }))
}

/** Rows from a list query carry their marks; keep them in the store. */
export function putRows(
  rows: { id: string; favourite: boolean; label: string | null }[]
): void {
  useMarks.getState().put(
    rows.map((r) => ({
      id: r.id,
      favourite: r.favourite,
      label: r.label as ColourLabel | null,
    }))
  )
}

/** The rail's Label section counts. */
export function listLabelCounts(): Promise<{ label: string; count: number }[]> {
  return invoke("list_label_counts")
}

/** Emitted by Rust after Undo or Redo changed the catalog. */
export const HISTORY_CHANGED = "history://changed"
