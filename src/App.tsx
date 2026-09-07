import { useCallback, useEffect, useMemo, useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { DropZone } from "./features/ingest/DropZone"
import { ImportConfirmation } from "./features/ingest/ImportConfirmation"
import { JobProgress } from "./features/ingest/JobProgress"
import { ContactSheet } from "./features/contact-sheet/ContactSheet"
import { LibrarySheet } from "./features/library/LibrarySheet"
import { TrashSheet } from "./features/trash/TrashSheet"
import { AppHeader } from "./components/shell/AppHeader"
import {
  DEFAULT_DIRECTION,
  LeftRail,
  LIBRARY_SORT_KEYS,
  TRASH_SORT_KEYS,
  type SortDirection,
  type SortKey,
} from "./components/shell/LeftRail"
import { usePersistedState } from "./lib/use-persisted-state"
import { SidePanel } from "./components/shell/SidePanel"
import {
  clampWidth,
  type PanelState,
} from "./components/shell/panel-geometry"
import { isTypingTarget } from "./lib/keyboard"
import { isOverlayOpen } from "./lib/overlay"
import {
  CARD_SIZE_DEFAULT,
  StatusBar,
} from "./components/shell/StatusBar"
import { VGA16_BUCKETS, type Vga16Bucket } from "./lib/vga16"
import { COLOUR_LABELS, type ColourLabel } from "./components/image-card/colour-label"
import { prescan, startIngest, type PrescanSummary } from "./features/ingest/api"
import {
  libraryCount,
  listBatches,
  listBucketCounts,
  type BatchSummary,
} from "./features/library/api"
import { useSelection } from "./stores/selection"
import { PropertiesPane } from "./features/properties/PropertiesPane"
import { SnackbarProvider } from "./components/ui/snackbar"
import { useMoveToTrash } from "./features/library/use-move-to-trash"

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

type Panel = "library" | "trash"

type LibraryCounts = {
  total: number
  buckets: Record<Vga16Bucket, number>
  batches: BatchSummary[]
  trashed: number
}

// Pure fetch — no state. Keeping it outside the component lets the mount
// effect apply the result in a promise callback instead of calling a
// setState-bearing function synchronously.
async function fetchLibraryCounts(): Promise<LibraryCounts> {
  const [total, counts, batches, trashed] = await Promise.all([
    libraryCount(),
    listBucketCounts(),
    listBatches(),
    libraryCount({ onlyDeleted: true }),
  ])
  const buckets = { ...EMPTY_BUCKET_COUNTS }
  for (const { bucket, count } of counts) buckets[bucket] = count
  return { total, buckets, batches, trashed }
}

// Left rail needs ~200px to render its longest section heading without
// ellipsis; the properties pane needs room for a label/value row.
const LEFT_PANEL = { default: 220, min: 180, max: 420 }
const RIGHT_PANEL = { default: 320, min: 240, max: 520 }

function AppShell() {
  const { selection, selectBatch, clear } = useSelection()
  const [view, setView] = useState<View>({ kind: "idle" })
  const [panel, setPanel] = useState<Panel>("library")
  const [sortState, setSortState] = usePersistedState<{
    key: SortKey
    direction: SortDirection
  }>("strata.library.sort", { key: "imported", direction: "desc" })
  const [trashDirection, setTrashDirection] =
    usePersistedState<SortDirection>("strata.trash.sortDirection", "desc")
  const onSortChange = useCallback(
    (key: SortKey) => {
      if (panel === "trash") return
      setSortState({ key, direction: DEFAULT_DIRECTION[key] })
    },
    [panel, setSortState],
  )
  const onDirectionToggle = useCallback(() => {
    if (panel === "trash") {
      setTrashDirection((prev) => (prev === "asc" ? "desc" : "asc"))
    } else {
      setSortState((prev) => ({
        ...prev,
        direction: prev.direction === "asc" ? "desc" : "asc",
      }))
    }
  }, [panel, setSortState, setTrashDirection])
  const [leftPanel, setLeftPanel] = usePersistedState<PanelState>(
    "strata.shell.panel.left",
    { width: LEFT_PANEL.default, collapsed: false },
  )
  const [rightPanel, setRightPanel] = usePersistedState<PanelState>(
    "strata.shell.panel.right",
    { width: RIGHT_PANEL.default, collapsed: false },
  )
  // A width persisted on a wider display must not escape this display's
  // bounds, so clamp on read rather than trusting what was stored.
  const leftWidth = clampWidth(leftPanel.width, LEFT_PANEL.min, LEFT_PANEL.max)
  const rightWidth = clampWidth(
    rightPanel.width,
    RIGHT_PANEL.min,
    RIGHT_PANEL.max,
  )
  const setLeftWidth = useCallback(
    (width: number) => setLeftPanel((p) => ({ ...p, width })),
    [setLeftPanel],
  )
  const setRightWidth = useCallback(
    (width: number) => setRightPanel((p) => ({ ...p, width })),
    [setRightPanel],
  )
  const toggleLeftPanel = useCallback(
    () => setLeftPanel((p) => ({ ...p, collapsed: !p.collapsed })),
    [setLeftPanel],
  )
  const toggleRightPanel = useCallback(
    () => setRightPanel((p) => ({ ...p, collapsed: !p.collapsed })),
    [setRightPanel],
  )
  const expandLeftPanel = useCallback(
    () => setLeftPanel((p) => ({ ...p, collapsed: false })),
    [setLeftPanel],
  )
  const expandRightPanel = useCallback(
    () => setRightPanel((p) => ({ ...p, collapsed: false })),
    [setRightPanel],
  )

  const [cardSize, setCardSize] = useState<number>(CARD_SIZE_DEFAULT)
  const [runningBatchId, setRunningBatchId] = useState<string | null>(null)
  const [selectedBuckets, setSelectedBuckets] = useState<Set<Vga16Bucket>>(
    () => new Set(),
  )
  const [selectedLabels, setSelectedLabels] = useState<Set<LabelSelector>>(
    () => new Set(),
  )
  const [bucketCounts, setBucketCounts] =
    useState<Record<Vga16Bucket, number>>(EMPTY_BUCKET_COUNTS)
  const [totalCount, setTotalCount] = useState(0)
  const [queriedCount, setQueriedCount] = useState<number | null>(null)
  const [batches, setBatches] = useState<BatchSummary[]>([])
  const [selectedBatchId, setSelectedBatchId] = useState<string | null>(null)
  const [trashCount, setTrashCount] = useState(0)
  const moveToTrash = useMoveToTrash()
  const onTrashDrop = useCallback(
    (ids: string[]) => {
      void moveToTrash(ids)
    },
    [moveToTrash],
  )

  // Rail navigation is the universal escape hatch from any in-progress
  // ingest view. The backend job continues regardless — leaving the view
  // doesn't cancel the import.
  const leaveIngestView = useCallback(() => {
    setView((v) => (v.kind === "idle" ? v : { kind: "idle" }))
  }, [])

  const handleSelectBatch = useCallback(
    (id: string | null) => {
      leaveIngestView()
      setSelectedBatchId(id)
      setPanel("library")
      if (id === null) {
        if (selection.kind === "batch") clear()
        return
      }
      selectBatch(id)
    },
    [leaveIngestView, selection.kind, selectBatch, clear],
  )

  const onSelectTrash = useCallback(() => {
    leaveIngestView()
    setPanel("trash")
    setSelectedBatchId(null)
    if (selection.kind !== "none") clear()
  }, [leaveIngestView, selection.kind, clear])

  const onSelectLibrary = useCallback(() => {
    leaveIngestView()
    setPanel("library")
    setSelectedBatchId(null)
    if (selection.kind === "batch") clear()
  }, [leaveIngestView, selection.kind, clear])

  const bucketsForQuery = useMemo(
    () => Array.from(selectedBuckets),
    [selectedBuckets],
  )

  const applyCounts = useCallback((c: LibraryCounts) => {
    setTotalCount(c.total)
    setBucketCounts(c.buckets)
    setBatches(c.batches)
    setTrashCount(c.trashed)
  }, [])

  // Imperative refresh, for event listeners and child callbacks.
  const refreshCounts = useCallback(async () => {
    try {
      applyCounts(await fetchLibraryCounts())
    } catch (e) {
      console.error("refreshCounts failed:", e)
    }
  }, [applyCounts])

  useEffect(() => {
    let cancelled = false
    fetchLibraryCounts()
      .then((c) => {
        if (!cancelled) applyCounts(c)
      })
      .catch((e) => console.error("initial counts failed:", e))
    return () => {
      cancelled = true
    }
  }, [applyCounts])

  useEffect(() => {
    const offs: UnlistenFn[] = []
    let cancelled = false
    ;(async () => {
      offs.push(
        await listen("ingest://batch-done", () => {
          setRunningBatchId(null)
          void refreshCounts()
        }),
      )
      offs.push(
        await listen("palette://backfill-done", () => {
          void refreshCounts()
        }),
      )
      offs.push(
        await listen("library://images-trashed", () => {
          void refreshCounts()
        }),
      )
      offs.push(
        await listen("library://images-restored", () => {
          void refreshCounts()
        }),
      )
      offs.push(
        await listen("library://images-purged", () => {
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

  const hasActiveQuery =
    bucketsForQuery.length > 0 || selectedBatchId !== null

  useEffect(() => {
    if (!hasActiveQuery) return
    let cancelled = false
    void libraryCount({ buckets: bucketsForQuery, batchId: selectedBatchId })
      .then((n) => {
        if (!cancelled) setQueriedCount(n)
      })
      .catch((e) => console.error("filtered count failed:", e))
    return () => {
      cancelled = true
    }
  }, [hasActiveQuery, bucketsForQuery, selectedBatchId])

  // Derived rather than stored: gating on hasActiveQuery keeps a count left
  // over from a previous filter from showing once the filter is cleared.
  const filteredCount = hasActiveQuery ? queriedCount : null

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
      setRunningBatchId(result.batch_id)
      setView({ kind: "running", batchId: result.batch_id })
    } catch (e) {
      setView({ kind: "error", message: String(e) })
    }
  }, [])

  const handleBack = useCallback(() => {
    setView({ kind: "idle" })
  }, [])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setView((v) => (v.kind === "idle" ? v : { kind: "idle" }))
        return
      }
      if (e.key !== "[" && e.key !== "]") return
      // Quickview owns the screen while open, and a typed "[" belongs to the
      // field, not the shell.
      if (isOverlayOpen() || isTypingTarget(e.target)) return
      if (e.metaKey || e.ctrlKey || e.altKey) return
      e.preventDefault()
      if (e.key === "[") toggleLeftPanel()
      else toggleRightPanel()
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [toggleLeftPanel, toggleRightPanel])

  const handleAdd = useCallback(async () => {
    const picked = await open({ directory: true, multiple: false })
    if (typeof picked === "string") {
      void handlePath(picked)
    }
  }, [handlePath])

  const toggleBucket = useCallback(
    (b: Vga16Bucket) => {
      leaveIngestView()
      setPanel("library")
      setSelectedBuckets((prev) => {
        const next = new Set(prev)
        if (next.has(b)) next.delete(b)
        else next.add(b)
        return next
      })
    },
    [leaveIngestView],
  )

  const toggleLabel = useCallback(
    (l: LabelSelector) => {
      leaveIngestView()
      setPanel("library")
      setSelectedLabels((prev) => {
        const next = new Set(prev)
        if (next.has(l)) next.delete(l)
        else next.add(l)
        return next
      })
    },
    [leaveIngestView],
  )

  const activeFilterCount =
    selectedBuckets.size + selectedLabels.size + (selectedBatchId ? 1 : 0)
  const addDisabled =
    view.kind === "scanning" ||
    view.kind === "scanned" ||
    runningBatchId !== null

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
    if (panel === "trash") {
      return (
        <TrashSheet
          direction={trashDirection}
          cellSize={cardSize}
          onLibraryChanged={refreshCounts}
        />
      )
    }
    return (
      <LibrarySheet
        sort={sortState.key}
        direction={sortState.direction}
        buckets={bucketsForQuery}
        batchId={selectedBatchId}
        cellSize={cardSize}
        onLibraryChanged={refreshCounts}
      />
    )
  }, [
    view,
    panel,
    handleConfirm,
    sortState,
    trashDirection,
    bucketsForQuery,
    selectedBatchId,
    cardSize,
    refreshCounts,
  ])

  return (
    <div className="flex h-svh flex-col">
      <DropZone onDropped={handlePath} />
      <AppHeader
        total={totalCount}
        filtered={filteredCount}
        activeFilterCount={activeFilterCount}
        onAdd={handleAdd}
        addDisabled={addDisabled}
        onBack={view.kind !== "idle" ? handleBack : undefined}
        leftCollapsed={leftPanel.collapsed}
        onToggleLeft={toggleLeftPanel}
        rightCollapsed={rightPanel.collapsed}
        onToggleRight={toggleRightPanel}
      />
      <div className="flex min-h-0 flex-1">
        <SidePanel
          side="left"
          title="Filters"
          width={leftWidth}
          onWidthChange={setLeftWidth}
          collapsed={leftPanel.collapsed}
          onExpand={expandLeftPanel}
          defaultWidth={LEFT_PANEL.default}
          minWidth={LEFT_PANEL.min}
          maxWidth={LEFT_PANEL.max}
        >
          <LeftRail
            bucketCounts={bucketCounts}
            selectedBuckets={selectedBuckets}
            onToggleBucket={toggleBucket}
            labelCounts={EMPTY_LABEL_COUNTS}
            selectedLabels={selectedLabels}
            onToggleLabel={toggleLabel}
            sort={panel === "trash" ? "deleted" : sortState.key}
            onSortChange={onSortChange}
            direction={panel === "trash" ? trashDirection : sortState.direction}
            onDirectionToggle={onDirectionToggle}
            sortOptions={panel === "trash" ? TRASH_SORT_KEYS : LIBRARY_SORT_KEYS}
            batches={batches}
            selectedBatchId={selectedBatchId}
            onSelectBatch={handleSelectBatch}
            trashCount={trashCount}
            onTrashDrop={onTrashDrop}
            trashActive={panel === "trash"}
            onSelectTrash={onSelectTrash}
            libraryTotal={totalCount}
            libraryActive={panel === "library"}
            onSelectLibrary={onSelectLibrary}
            filtersDisabled={panel === "trash"}
          />
        </SidePanel>
        <main className="flex min-w-0 flex-1 flex-col gap-3 p-4">{content}</main>
        <SidePanel
          side="right"
          title="Properties"
          width={rightWidth}
          onWidthChange={setRightWidth}
          collapsed={rightPanel.collapsed}
          onExpand={expandRightPanel}
          defaultWidth={RIGHT_PANEL.default}
          minWidth={RIGHT_PANEL.min}
          maxWidth={RIGHT_PANEL.max}
        >
          <PropertiesPane />
        </SidePanel>
      </div>
      <StatusBar cardSize={cardSize} onCardSizeChange={setCardSize} />
    </div>
  )
}

export default function App() {
  return (
    <SnackbarProvider>
      <AppShell />
    </SnackbarProvider>
  )
}
