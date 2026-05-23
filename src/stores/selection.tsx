/* eslint-disable react-refresh/only-export-components --
 * Provider + hook colocation is idiomatic; splitting them across files for
 * fast-refresh purity is over-organisation for a 30-line store. */
import { createContext, useCallback, useContext, useMemo, useState } from "react"
import type { ImportedRow } from "@/features/contact-sheet/api"
import type { BatchSummary } from "@/features/library/api"

export type Selection =
  | { kind: "none" }
  | { kind: "image"; id: string; row: ImportedRow }
  | { kind: "batch"; id: string; batch: BatchSummary }

interface SelectionContextValue {
  selection: Selection
  selectImage: (row: ImportedRow) => void
  selectBatch: (batch: BatchSummary | null) => void
  clear: () => void
}

const SelectionContext = createContext<SelectionContextValue | null>(null)

export function SelectionProvider({ children }: { children: React.ReactNode }) {
  const [selection, setSelection] = useState<Selection>({ kind: "none" })

  const selectImage = useCallback((row: ImportedRow) => {
    setSelection({ kind: "image", id: row.id, row })
  }, [])

  const selectBatch = useCallback((batch: BatchSummary | null) => {
    if (!batch) {
      setSelection({ kind: "none" })
      return
    }
    setSelection({ kind: "batch", id: batch.id, batch })
  }, [])

  const clear = useCallback(() => setSelection({ kind: "none" }), [])

  const value = useMemo(
    () => ({ selection, selectImage, selectBatch, clear }),
    [selection, selectImage, selectBatch, clear],
  )

  return (
    <SelectionContext.Provider value={value}>
      {children}
    </SelectionContext.Provider>
  )
}

export function useSelection(): SelectionContextValue {
  const ctx = useContext(SelectionContext)
  if (!ctx) {
    throw new Error("useSelection must be used inside <SelectionProvider>")
  }
  return ctx
}
