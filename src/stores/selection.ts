import { create } from "zustand"

export type Selection =
  | { kind: "none" }
  | { kind: "image"; id: string }
  | { kind: "images"; ids: string[] }
  | { kind: "batch"; id: string }

interface SelectionStore {
  selection: Selection
  /** The image a Shift+click range grows from: the last one clicked. */
  anchor: string | null
  selectImage: (id: string) => void
  /** Cmd+click: add the image, or drop it when already selected. */
  toggleImage: (id: string) => void
  /** Shift+click: these ids, in grid order. Keeps the anchor. */
  selectImages: (ids: string[]) => void
  selectBatch: (id: string) => void
  clear: () => void
}

/** Zero ids is none, one is `image`, so single-image consumers stay as they were. */
function fromIds(ids: string[]): Selection {
  if (ids.length === 0) return { kind: "none" }
  if (ids.length === 1) return { kind: "image", id: ids[0] }
  return { kind: "images", ids }
}

/** The selected image ids, whether one or several; empty for none and a batch. */
export function selectedImageIds(selection: Selection): string[] {
  if (selection.kind === "image") return [selection.id]
  if (selection.kind === "images") return selection.ids
  return []
}

export const useSelection = create<SelectionStore>()((set) => ({
  selection: { kind: "none" },
  anchor: null,
  selectImage: (id) => set({ selection: { kind: "image", id }, anchor: id }),
  toggleImage: (id) =>
    set((s) => {
      const ids = selectedImageIds(s.selection)
      const next = ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id]
      return { selection: fromIds(next), anchor: id }
    }),
  selectImages: (ids) => set({ selection: fromIds(ids) }),
  selectBatch: (id) => set({ selection: { kind: "batch", id }, anchor: null }),
  clear: () => set({ selection: { kind: "none" }, anchor: null }),
}))
