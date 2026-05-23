import { useCallback, useEffect, useMemo, useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { DropZone } from "./features/ingest/DropZone"
import { ImportConfirmation } from "./features/ingest/ImportConfirmation"
import { JobProgress } from "./features/ingest/JobProgress"
import { ContactSheet } from "./features/contact-sheet/ContactSheet"
import { LibrarySheet } from "./features/library/LibrarySheet"
import { AppHeader } from "./components/shell/AppHeader"
import { LeftRail, type SortKey } from "./components/shell/LeftRail"
import {
  CARD_SIZE_DEFAULT,
  StatusBar,
} from "./components/shell/StatusBar"
import { VGA16_BUCKETS, type Vga16Bucket } from "./lib/vga16"
import { COLOUR_LABELS, type ColourLabel } from "./components/image-card/colour-label"
import { prescan, startIngest, type PrescanSummary } from "./features/ingest/api"
import { libraryCount, listBucketCounts } from "./features/library/api"

type View =
  | { kind: "idle" }
  | { kind: "scanning"; path: string }
  | { kind: "scanned"; scan: PrescanSummary }
  | { kind: "running"; batchId: string }
  | { kind: "error"; message: string }

type LabelSelector = "favourite" | ColourLabel

const EMPTY_BUCKET_COUNTS: Record<Vga16Bucket, number> = Object.fromEntries(
  VGA16_BUCKETS.map((b) => [b, 0]),
) as Record<Vga16Bucket, number>

const EMPTY_LABEL_COUNTS: Record<LabelSelector, number> = {
  favourite: 0,
  ...(Object.fromEntries(COLOUR_LABELS.map((c) => [c, 0])) as Record<
    ColourLabel,
    number
  >),
}

function App() {
  const [view, setView] = useState<View>({ kind: "idle" })
  const [sort, setSort] = useState<SortKey>("imported")
  const [cardSize, setCardSize] = useState<number>(CARD_SIZE_DEFAULT)
  const [selectedBuckets, setSelectedBuckets] = useState<Set<Vga16Bucket>>(
    () => new Set(),
  )
  const [selectedLabels, setSelectedLabels] = useState<Set<LabelSelector>>(
    () => new Set(),
  )
  const [bucketCounts, setBucketCounts] =
    useState<Record<Vga16Bucket, number>>(EMPTY_BUCKET_COUNTS)
  const [totalCount, setTotalCount] = useState(0)
  const [filteredCount, setFilteredCount] = useState<number | null>(null)

  const bucketsForQuery = useMemo(
    () => Array.from(selectedBuckets),
    [selectedBuckets],
  )

  const refreshCounts = useCallback(async () => {
    try {
      const [total, counts] = await Promise.all([
        libraryCount(),
        listBucketCounts(),
      ])
      setTotalCount(total)
      const next = { ...EMPTY_BUCKET_COUNTS }
      for (const { bucket, count } of counts) next[bucket] = count
      setBucketCounts(next)
    } catch (e) {
      console.error("refreshCounts failed:", e)
    }
  }, [])

  useEffect(() => {
    void refreshCounts()
  }, [refreshCounts])

  useEffect(() => {
    const offs: UnlistenFn[] = []
    let cancelled = false
    ;(async () => {
      offs.push(
        await listen("ingest://batch-done", () => {
          void refreshCounts()
        }),
      )
      offs.push(
        await listen("palette://backfill-done", () => {
          void refreshCounts()
        }),
      )
      if (cancelled) offs.forEach((o) => o())
    })()
    return () => {
      cancelled = true
      offs.forEach((o) => o())
    }
  }, [refreshCounts])

  useEffect(() => {
    if (bucketsForQuery.length === 0) {
      setFilteredCount(null)
      return
    }
    let cancelled = false
    void libraryCount(bucketsForQuery)
      .then((n) => {
        if (!cancelled) setFilteredCount(n)
      })
      .catch((e) => console.error("filtered count failed:", e))
    return () => {
      cancelled = true
    }
  }, [bucketsForQuery])

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

  const handleAdd = useCallback(async () => {
    const picked = await open({ directory: true, multiple: false })
    if (typeof picked === "string") {
      void handlePath(picked)
    }
  }, [handlePath])

  const toggleBucket = useCallback((b: Vga16Bucket) => {
    setSelectedBuckets((prev) => {
      const next = new Set(prev)
      if (next.has(b)) next.delete(b)
      else next.add(b)
      return next
    })
  }, [])

  const toggleLabel = useCallback((l: LabelSelector) => {
    setSelectedLabels((prev) => {
      const next = new Set(prev)
      if (next.has(l)) next.delete(l)
      else next.add(l)
      return next
    })
  }, [])

  const activeFilterCount = selectedBuckets.size + selectedLabels.size
  const addDisabled = view.kind === "scanning" || view.kind === "running"

  const content = useMemo(() => {
    if (view.kind === "scanning") {
      return (
        <p className="text-sm text-muted-foreground">
          Scanning <span className="select-text">{view.path}</span>...
        </p>
      )
    }
    if (view.kind === "error") {
      return (
        <p className="text-sm text-destructive">
          Error: <span className="select-text">{view.message}</span>
        </p>
      )
    }
    if (view.kind === "scanned") {
      return (
        <ImportConfirmation
          scan={view.scan}
          onConfirm={() => handleConfirm(view.scan)}
          onCancel={() => setView({ kind: "idle" })}
        />
      )
    }
    if (view.kind === "running") {
      return (
        <>
          <JobProgress batchId={view.batchId} />
          <ContactSheet batchId={view.batchId} cellSize={cardSize} />
        </>
      )
    }
    return (
      <LibrarySheet
        sort={sort}
        buckets={bucketsForQuery}
        cellSize={cardSize}
        onLibraryChanged={refreshCounts}
      />
    )
  }, [view, handleConfirm, sort, bucketsForQuery, cardSize, refreshCounts])

  return (
    <div className="flex h-svh flex-col">
      <DropZone onDropped={handlePath} />
      <AppHeader
        total={totalCount}
        filtered={filteredCount}
        activeFilterCount={activeFilterCount}
        onAdd={handleAdd}
        addDisabled={addDisabled}
      />
      <div className="flex min-h-0 flex-1">
        <LeftRail
          bucketCounts={bucketCounts}
          selectedBuckets={selectedBuckets}
          onToggleBucket={toggleBucket}
          labelCounts={EMPTY_LABEL_COUNTS}
          selectedLabels={selectedLabels}
          onToggleLabel={toggleLabel}
          sort={sort}
          onSortChange={setSort}
        />
        <main className="flex min-w-0 flex-1 flex-col gap-3 p-4">{content}</main>
      </div>
      <StatusBar cardSize={cardSize} onCardSizeChange={setCardSize} />
    </div>
  )
}

export default App
