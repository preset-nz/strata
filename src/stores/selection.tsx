import { create } from "zustand"

export type Selection =
  | { kind: "none" }
  | { kind: "image"; id: string }
  | { kind: "batch"; id: string }

interface SelectionStore {
  selection: Selection
  selectImage: (id: string) => void
  selectBatch: (id: string) => void
  clear: () => void
}

export const useSelection = create<SelectionStore>()((set) => ({
  selection: { kind: "none" },
  selectImage: (id) => set({ selection: { kind: "image", id } }),
  selectBatch: (id) => set({ selection: { kind: "batch", id } }),
  clear: () => set({ selection: { kind: "none" } }),
}))
