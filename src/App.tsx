import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { DropZone } from "./features/ingest/DropZone"
import { ImportConfirmation } from "./features/ingest/ImportConfirmation"
import { JobProgress } from "./features/ingest/JobProgress"
import { ContactSheet } from "./features/contact-sheet/ContactSheet"
import { LibrarySheet } from "./features/library/LibrarySheet"
import { TrashSheet } from "./features/trash/TrashSheet"
import { AppHeader } from "./components/shell/AppHeader"
import { LeftRail } from "./components/shell/LeftRail"
import {
  DEFAULT_DIRECTION,
  LIBRARY_SORT_KEYS,
  TRASH_SORT_KEYS,
  type SortDirection,
  type SortKey,
} from "./components/shell/sort-keys"
import {
  onSettingsMenu,
  SettingsWindow,
  usePersistedState,
  usePreference,
  usePreferences,
  usePreferencesBootstrap,
} from "@preset.nz/preferences"
import { SidePanel } from "./components/shell/SidePanel"
import {
  clampWidth,
  type PanelState,
} from "./components/shell/panel-geometry"
import { isTypingTarget } from "./lib/keyboard"
import { isOverlayOpen, registerOverlay } from "./lib/overlay"
import { ThemeSync } from "./components/theme-sync"
import {
  CARD_SIZE_DEFAULT,
  StatusBar,
} from "./components/shell/StatusBar"
import { VGA16_BUCKETS, type Vga16Bucket } from "./lib/vga16"
import { ORIENTATIONS, type Orientation } from "./lib/orientation"
import { COLOUR_LABELS, type ColourLabel } from "./components/image-card/colour-label"
import { prescan, startIngest, type PrescanSummary } from "./features/ingest/api"
import {
  libraryCount,
  listBatches,
  listBucketCounts,
  listOrientationCounts,
  type BatchSummary,
} from "./features/library/api"
import { useSelection } from "./stores/selection"
import { PropertiesPane } from "./features/properties/PropertiesPane"
import { SnackbarProvider } from "./components/ui/snackbar"
import { useMoveToTrash } from "./features/library/use-move-to-trash"
import { SavedSearchList } from "./features/saved-searches/SavedSearchList"
import { makeQuery, type SavedQuery } from "./features/saved-searches/query"
import { useSavedSearches } from "./features/saved-searches/use-saved-searches"
import {
  HISTORY_CHANGED,
  listLabelCounts,
  setColourLabel,
  setFavourite,
  useMark,
} from "./features/curation/marks"
import { useCommands, useTextFocus, type Binding } from "@preset.nz/app-kit/core"
import { CollectionList } from "./features/collections/CollectionList"
import { useCollections } from "./features/collections/use-collections"
import { NewProjectDialog } from "./features/projects/NewProjectDialog"
import { ProjectList } from "./features/projects/ProjectList"
import { useProjects } from "./features/projects/use-projects"
import type { GroupMenu } from "./components/image-card/CardContextMenu"
import { paletteMarkers, usePaletteMarkers } from "./features/quickview/palette-markers"

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

const EMPTY_ORIENTATION_COUNTS: Record<Orientation, number> =
  Object.fromEntries(ORIENTATIONS.map((o) => [o, 0])) as Record<
    Orientation,
    number
  >

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
  orientations: Record<Orientation, number>
  batches: BatchSummary[]
  trashed: number
  labels: Record<LabelSelector, number>
}

// Pure fetch — no state. Keeping it outside the component lets the mount
// effect apply the result in a promise callback instead of calling a
// setState-bearing function synchronously.
async function fetchLibraryCounts(): Promise<LibraryCounts> {
  const [total, counts, orientationRows, batches, trashed, labelRows] = await Promise.all([
    libraryCount(),
    listBucketCounts(),
    listOrientationCounts(),
    listBatches(),
    libraryCount({ onlyDeleted: true }),
    listLabelCounts(),
  ])
  const buckets = { ...EMPTY_BUCKET_COUNTS }
  for (const { bucket, count } of counts) buckets[bucket] = count
  const orientations = { ...EMPTY_ORIENTATION_COUNTS }
  for (const { orientation, count } of orientationRows) {
    orientations[orientation] = count
  }
  const labels = { ...EMPTY_LABEL_COUNTS }
  for (const { label, count } of labelRows) {
    if (label in labels) labels[label as LabelSelector] = count
  }
  return { total, buckets, orientations, batches, trashed, labels }
}

// Left rail needs ~200px to render its longest section heading without
// ellipsis; the properties pane needs room for a label/value row.
const LEFT_PANEL = { default: 220, min: 180, max: 420 }
const RIGHT_PANEL = { default: 320, min: 240, max: 520 }

// Whether a sort was remembered from a previous session, read before the
// persisted hook writes one. Only a fresh install takes the preference.
function hadStoredSort(): boolean {
  try {
    return localStorage.getItem("strata.library.sort") !== null
  } catch {
    return true
  }
}

function AppShell() {
  usePreferencesBootstrap()
  const { selection, selectBatch, clear } = useSelection()
  const [view, setView] = useState<View>({ kind: "idle" })
  const [panel, setPanel] = useState<Panel>("library")
  const [sortState, setSortState] = usePersistedState<{
    key: SortKey
    direction: SortDirection
  }>("strata.library.sort", { key: "imported", direction: "desc" })
  const [freshSort] = useState(() => !hadStoredSort())
  const defaultSort = usePreference<string>("library.default_sort", "imported")
  const prefsLoaded = usePreferences() !== null
  useEffect(() => {
    if (!freshSort || !prefsLoaded) return
    const key = defaultSort as SortKey
    if (!(key in DEFAULT_DIRECTION)) return
    setSortState((prev) =>
      prev.key === key ? prev : { key, direction: DEFAULT_DIRECTION[key] },
    )
    // Runs once, when preferences first arrive on a fresh install.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [prefsLoaded])

  const [settingsOpen, setSettingsOpen] = useState(false)
  useEffect(() => onSettingsMenu(() => setSettingsOpen(true)), [])
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
  const [labelCounts, setLabelCounts] = useState(EMPTY_LABEL_COUNTS)
  const [selectedLabels, setSelectedLabels] = useState<Set<LabelSelector>>(
    () => new Set(),
  )
  const [bucketCounts, setBucketCounts] =
    useState<Record<Vga16Bucket, number>>(EMPTY_BUCKET_COUNTS)
  const [selectedOrientations, setSelectedOrientations] = useState<
    Set<Orientation>
  >(() => new Set())
  const [orientationCounts, setOrientationCounts] = useState<
    Record<Orientation, number>
  >(EMPTY_ORIENTATION_COUNTS)
  const [totalCount, setTotalCount] = useState(0)
  const [queriedCount, setQueriedCount] = useState<number | null>(null)
  // What is typed, and what the library is queried with: the second trails
  // the first so each keystroke doesn't refetch.
  const [searchText, setSearchText] = useState("")
  const [searchQuery, setSearchQuery] = useState("")
  const searchRef = useRef<HTMLInputElement | null>(null)
  const [batches, setBatches] = useState<BatchSummary[]>([])
  const [selectedBatchId, setSelectedBatchId] = useState<string | null>(null)
  const [selectedCollectionId, setSelectedCollectionId] = useState<string | null>(null)
  const [selectedProjectKey, setSelectedProjectKey] = useState<string | null>(null)
  // Bumped when membership changed under the library's feet (added to or
  // removed from the collection it shows).
  const [reloadToken, setReloadToken] = useState(0)
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
  const orientationsForQuery = useMemo(
    () => Array.from(selectedOrientations),
    [selectedOrientations],
  )
  const labelsForQuery = useMemo(() => Array.from(selectedLabels), [selectedLabels])

  const applyCounts = useCallback((c: LibraryCounts) => {
    setTotalCount(c.total)
    setBucketCounts(c.buckets)
    setOrientationCounts(c.orientations)
    setBatches(c.batches)
    setTrashCount(c.trashed)
    setLabelCounts(c.labels)
    // list_batches hides batches that no longer have any live images, so a
    // batch can vanish from the rail while it is still the active filter —
    // trash its last image and the row backing the filter is gone. Drop the
    // selection with it, or the catalog shows an empty grid and a filter badge
    // with nothing in the rail to explain or clear them.
    setSelectedBatchId((prev) =>
      prev && c.batches.some((b) => b.id === prev) ? prev : null,
    )
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
        // Undo and Redo change the catalog behind every view: counts, the
        // grid (through reloadToken) and the filtered count.
        await listen(HISTORY_CHANGED, () => {
          void refreshCounts()
          setReloadToken((t) => t + 1)
        }),
      )
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
        await listen("orientation://backfill-done", () => {
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

  useEffect(() => {
    const t = setTimeout(() => setSearchQuery(searchText.trim()), 150)
    return () => clearTimeout(t)
  }, [searchText])

  const currentQuery = useMemo(
    () =>
      makeQuery({
        text: searchQuery,
        buckets: bucketsForQuery,
        orientations: orientationsForQuery,
        labels: labelsForQuery,
        batchId: selectedBatchId,
        collectionId: selectedCollectionId,
        projectKey: selectedProjectKey,
        sort: sortState.key,
        direction: sortState.direction,
      }),
    [searchQuery, bucketsForQuery, orientationsForQuery, labelsForQuery, selectedBatchId, selectedCollectionId, selectedProjectKey, sortState],
  )
  const applyQuery = useCallback(
    (q: SavedQuery) => {
      leaveIngestView()
      setPanel("library")
      setSearchText(q.text)
      setSearchQuery(q.text)
      setSelectedBuckets(new Set(q.buckets))
      setSelectedOrientations(new Set(q.orientations))
      setSelectedLabels(new Set(q.labels))
      setSelectedBatchId(q.batchId)
      setSelectedCollectionId(q.collectionId)
      setSelectedProjectKey(q.projectKey)
      setSortState({ key: q.sort, direction: q.direction })
    },
    [leaveIngestView, setSortState],
  )
  const saved = useSavedSearches(currentQuery, applyQuery)

  const collectionChanged = useCallback(() => {
    void refreshCounts()
    setReloadToken((t) => t + 1)
  }, [refreshCounts])
  const coll = useCollections(collectionChanged)
  // A deleted collection can't stay the filter.
  useEffect(() => {
    if (selectedCollectionId && !coll.collections.some((c) => c.id === selectedCollectionId)) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- follows the list
      setSelectedCollectionId(null)
    }
  }, [coll.collections, selectedCollectionId])
  const proj = useProjects(collectionChanged)
  // A project archived elsewhere stays selectable; one whose folder is gone can't stay the filter.
  useEffect(() => {
    if (selectedProjectKey && proj.projects.length > 0 && !proj.projects.some((p) => p.key === selectedProjectKey)) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- follows the list
      setSelectedProjectKey(null)
    }
  }, [proj.projects, selectedProjectKey])
  const groupMenus = useCallback(
    (imageId: string): GroupMenu[] => [
      {
        title: "Collection",
        items: coll.collections,
        onAdd: (id) => void coll.add(id, [imageId]),
        onNew: () => coll.startNaming([imageId]),
        onRemove: selectedCollectionId ? () => void coll.removeImages(selectedCollectionId, [imageId]) : undefined,
      },
      {
        title: "Project",
        items: proj.projects.filter((p) => !p.archived).map((p) => ({ id: p.key, name: p.name })),
        onAdd: (key) => void proj.add(key, [imageId]),
        onNew: proj.startCreating,
        onRemove: selectedProjectKey ? () => void proj.removeImages(selectedProjectKey, [imageId]) : undefined,
      },
    ],
    [coll, proj, selectedCollectionId, selectedProjectKey],
  )

  // The menu's commands, by the ids `src-tauri/src/menu.rs` declares. app-kit
  // owns Undo and Redo, keeps enabled and checked state in step with the
  // menu, and sends Cmd+Z to a focused text field.
  const markers = usePaletteMarkers()
  const { startNaming } = saved
  // Image menu: acts on the selected image. Off while typing, so the bare
  // F and 0-7 keys reach the text field instead.
  const textFocus = useTextFocus()
  const selectedImage = selection.kind === "image" && panel === "library" ? selection.id : null
  const selectedMark = useMark(selectedImage)
  const canMark = selectedImage !== null && !textFocus
  const bindings = useMemo<Record<string, Binding>>(
    () => ({
      "image.favourite": {
        enabled: canMark,
        pressed: selectedMark.favourite,
        run: () => {
          if (selectedImage) void setFavourite([selectedImage], !selectedMark.favourite).then(refreshCounts)
        },
      },
      "file.new_project": { run: proj.startCreating },
      "image.new_collection": {
        enabled: !textFocus,
        run: () => coll.startNaming(selectedImage ? [selectedImage] : []),
      },
      "image.remove_from_collection": {
        enabled: canMark && selectedCollectionId !== null,
        run: () => {
          if (selectedImage && selectedCollectionId) void coll.removeImages(selectedCollectionId, [selectedImage])
        },
      },
      "image.label.none": {
        enabled: canMark && selectedMark.label !== null,
        run: () => {
          if (selectedImage) void setColourLabel([selectedImage], null).then(refreshCounts)
        },
      },
      ...Object.fromEntries(
        COLOUR_LABELS.map((c) => [
          `image.label.${c}`,
          {
            enabled: canMark,
            run: () => {
              if (selectedImage) void setColourLabel([selectedImage], c).then(refreshCounts)
            },
          },
        ]),
      ),
      "edit.find": {
        run: () => {
          searchRef.current?.focus()
          searchRef.current?.select()
        },
      },
      "edit.save_search": { run: startNaming },
      "view.palette_markers": { pressed: markers.on, run: paletteMarkers.toggle },
    }),
    [startNaming, markers.on, canMark, selectedImage, selectedMark, refreshCounts, textFocus, coll, selectedCollectionId, proj.startCreating],
  )
  useCommands(bindings)

  const hasActiveQuery =
    searchQuery !== "" ||
    bucketsForQuery.length > 0 ||
    orientationsForQuery.length > 0 ||
    labelsForQuery.length > 0 ||
    selectedBatchId !== null ||
    selectedCollectionId !== null ||
    selectedProjectKey !== null

  useEffect(() => {
    if (!hasActiveQuery) return
    let cancelled = false
    void libraryCount({
      buckets: bucketsForQuery,
      orientations: orientationsForQuery,
      labels: labelsForQuery,
      batchId: selectedBatchId,
      collectionId: selectedCollectionId,
      projectKey: selectedProjectKey,
      query: searchQuery,
    })
      .then((n) => {
        if (!cancelled) setQueriedCount(n)
      })
      .catch((e) => console.error("filtered count failed:", e))
    return () => {
      cancelled = true
    }
    // reloadToken: membership changed under the same filter, so the count did too.
  }, [hasActiveQuery, bucketsForQuery, orientationsForQuery, labelsForQuery, selectedBatchId, selectedCollectionId, selectedProjectKey, searchQuery, reloadToken])

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

  const handleConfirm = useCallback(async (scan: PrescanSummary, folders: string[]) => {
    try {
      const result = await startIngest(scan.root, folders)
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

  const toggleOrientation = useCallback(
    (o: Orientation) => {
      leaveIngestView()
      setPanel("library")
      setSelectedOrientations((prev) => {
        const next = new Set(prev)
        if (next.has(o)) next.delete(o)
        else next.add(o)
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
    selectedBuckets.size +
    selectedOrientations.size +
    selectedLabels.size +
    (selectedBatchId ? 1 : 0) +
    (selectedCollectionId ? 1 : 0) +
    (selectedProjectKey ? 1 : 0)
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
          onConfirm={(folders) => handleConfirm(view.scan, folders)}
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
        orientations={orientationsForQuery}
        labels={labelsForQuery}
        batchId={selectedBatchId}
        collectionId={selectedCollectionId}
        projectKey={selectedProjectKey}
        reloadToken={reloadToken}
        groupMenus={groupMenus}
        query={searchQuery}
        cellSize={cardSize}
        onLibraryChanged={refreshCounts}
      />
    )
  }, [
    searchQuery,
    labelsForQuery,
    selectedCollectionId,
    selectedProjectKey,
    reloadToken,
    groupMenus,
    view,
    panel,
    handleConfirm,
    sortState,
    trashDirection,
    bucketsForQuery,
    orientationsForQuery,
    selectedBatchId,
    cardSize,
    refreshCounts,
  ])

  return (
    <div className="flex h-svh flex-col">
      <DropZone onDropped={handlePath} />
      {proj.creating && (
        <NewProjectDialog
          root="~/preset-nz/Projects"
          onCancel={proj.cancelCreating}
          onCreate={(name, description) =>
            void proj.create(name, description).then((key) => {
              if (key) {
                leaveIngestView()
                setPanel("library")
                setSelectedProjectKey(key)
              }
            })
          }
        />
      )}
      <ThemeSync />
      <SettingsWindow
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        title="Settings"
        onOverlay={registerOverlay}
      />
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
        search={searchText}
        onSearchChange={setSearchText}
        searchRef={searchRef}
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
            orientationCounts={orientationCounts}
            selectedOrientations={selectedOrientations}
            onToggleOrientation={toggleOrientation}
            bucketCounts={bucketCounts}
            selectedBuckets={selectedBuckets}
            onToggleBucket={toggleBucket}
            labelCounts={labelCounts}
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
            projectCount={proj.projects.filter((p) => !p.archived).length}
            projects={
              <ProjectList
                projects={proj.projects}
                selectedKey={selectedProjectKey}
                disabled={panel === "trash"}
                onSelect={(key) => {
                  leaveIngestView()
                  setPanel("library")
                  setSelectedProjectKey(key)
                }}
                onFavourite={(p, on) => void proj.setFavourite(p, on)}
                onArchive={(p, on) => void proj.setArchived(p, on)}
                onDrop={(key, images) => void proj.add(key, images)}
              />
            }
            collectionCount={coll.collections.length}
            collectionsOpen={coll.naming !== null}
            collections={
              <CollectionList
                collections={coll.collections}
                selectedId={selectedCollectionId}
                disabled={panel === "trash"}
                onSelect={(id) => {
                  leaveIngestView()
                  setPanel("library")
                  setSelectedCollectionId(id)
                }}
                onRename={(id, name) => void coll.rename(id, name)}
                onDelete={(c) => void coll.remove(c)}
                onDrop={(id, images) => void coll.add(id, images)}
                naming={coll.naming !== null}
                onName={(name) => void coll.create(name)}
                onCancelNaming={coll.cancelNaming}
              />
            }
            savedSearchCount={saved.searches.length}
            savedSearchesOpen={saved.naming !== null}
            savedSearches={
              <SavedSearchList
                searches={saved.searches}
                activeId={panel === "library" ? saved.activeId : null}
                onApply={saved.applySearch}
                onDelete={(s) => void saved.remove(s)}
                naming={saved.naming}
                onName={(name) => void saved.save(name)}
                onCancelNaming={saved.cancelNaming}
              />
            }
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
