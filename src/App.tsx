import { useCallback, useState } from "react"
import { FolderPicker } from "./features/ingest/FolderPicker"
import { DropZone } from "./features/ingest/DropZone"
import { ImportConfirmation } from "./features/ingest/ImportConfirmation"
import { JobProgress } from "./features/ingest/JobProgress"
import { ContactSheet } from "./features/contact-sheet/ContactSheet"
import { LibrarySheet } from "./features/library/LibrarySheet"
import { prescan, startIngest, type PrescanSummary } from "./features/ingest/api"

type View =
  | { kind: "idle" }
  | { kind: "scanning"; path: string }
  | { kind: "scanned"; scan: PrescanSummary }
  | { kind: "running"; batchId: string }
  | { kind: "error"; message: string }

function App() {
  const [view, setView] = useState<View>({ kind: "idle" })

  const handlePath = useCallback(async (path: string) => {
    setView({ kind: "scanning", path })
    try {
      const scan = await prescan(path)
      setView({ kind: "scanned", scan })
    } catch (e) {
      setView({ kind: "error", message: String(e) })
    }
  }, [])

  const handleConfirm = useCallback(async (scan: PrescanSummary) => {
    try {
      const result = await startIngest(scan.root)
      setView({ kind: "running", batchId: result.batch_id })
    } catch (e) {
      setView({ kind: "error", message: String(e) })
    }
  }, [])

  return (
    <main className="min-h-svh p-4">
      <h1 className="mt-0 mb-4 font-heading text-xl font-medium">Strata</h1>
      <DropZone onDropped={handlePath} />
      <div className="mb-4 flex items-center gap-2">
        <FolderPicker
          onPicked={handlePath}
          disabled={view.kind === "scanning" || view.kind === "running"}
        />
        <button
          type="button"
          className="rounded-sm border border-border bg-background px-2.5 py-1 text-xs hover:bg-muted"
          onClick={() => setView({ kind: "idle" })}
        >
          New import
        </button>
        <span className="text-xs text-muted-foreground">
          or drop a folder onto this window
        </span>
      </div>

      {view.kind === "scanning" && (
        <p className="text-sm text-muted-foreground">Scanning {view.path}...</p>
      )}
      {view.kind === "error" && (
        <p className="text-sm text-destructive">Error: {view.message}</p>
      )}
      {view.kind === "scanned" && (
        <ImportConfirmation
          scan={view.scan}
          onConfirm={() => handleConfirm(view.scan)}
          onCancel={() => setView({ kind: "idle" })}
        />
      )}
      {view.kind === "running" && (
        <>
          <JobProgress batchId={view.batchId} />
          <ContactSheet batchId={view.batchId} />
        </>
      )}
      {view.kind === "idle" && <LibrarySheet />}
    </main>
  )
}

export default App
