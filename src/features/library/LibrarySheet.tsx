import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { useCallback, useEffect, useRef, useState } from "react"
import {
  CardContextMenu,
  type GroupMenu,
} from "@/components/image-card/CardContextMenu"
import type { SortKey } from "@/components/shell/sort-keys"
import { SnackbarViewport } from "@/components/ui/snackbar"
import { MarkedImageCard } from "@/features/curation/MarkedImageCard"
import { putRows } from "@/features/curation/marks"
import { Quickview } from "@/features/quickview/Quickview"
import type { Orientation } from "@/lib/orientation"
import type { Vga16Bucket } from "@/lib/vga16"
import { selectedImageIds, useSelection } from "@/stores/selection"
import type { ImportedRow } from "../contact-sheet/api"
import { ThumbGrid } from "../contact-sheet/ThumbGrid"
import { listImages, type SortDirection } from "./api"
import { useMoveToTrash } from "./use-move-to-trash"

type Cell = {
  key: string
  hash: string
  filename: string
  status: string
  row: ImportedRow
}

const PAGE_SIZE = 100

function toCell(row: ImportedRow): Cell {
  return {
    key: row.id,
    hash: row.content_hash,
    filename: row.title,
    status: row.thumbnails_status,
    row,
  }
}

type Props = {
  sort: SortKey
  direction: SortDirection
  buckets: Vga16Bucket[]
  orientations: Orientation[]
  batchId?: string | null
  /** "favourite" and colour labels, any of. */
  labels?: string[]
  /** Show only this collection's images. */
  collectionId?: string | null
  query?: string
  /** Change it to reload: membership changed, or Undo / Redo ran. */
  reloadToken?: number
  /** The context menu's Add to … submenus, given the images it acts on. */
  groupMenus?: (imageIds: string[]) => GroupMenu[]
  /** Show only this project's images. */
  projectKey?: string | null
  cellSize?: number
  onLibraryChanged?: () => void
}

export function LibrarySheet({
  sort,
  direction,
  buckets,
  orientations,
  batchId,
  labels,
  collectionId,
  query,
  reloadToken,
  groupMenus,
  projectKey,
  cellSize,
  onLibraryChanged,
}: Props) {
  const [items, setItems] = useState<Cell[]>([])
  const [hasMore, setHasMore] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [quickviewIndex, setQuickviewIndex] = useState<number | null>(null)
  const loadingRef = useRef(false)
  const offsetRef = useRef(0)
  const queryRef = useRef({
    sort,
    direction,
    buckets,
    orientations,
    batchId,
    labels,
    collectionId,
    projectKey,
    query,
  })
  const pendingUndoRef = useRef<Map<string, { cell: Cell; idx: number }>>(
    new Map()
  )
  const { selection, selectImage, toggleImage, selectImages } = useSelection()
  const selectedIds = selectedImageIds(selection)
  const moveToTrash = useMoveToTrash()

  const loadNext = useCallback(async () => {
    if (loadingRef.current || !hasMore) return
    loadingRef.current = true
    try {
      const opts = queryRef.current
      const rows = await listImages(offsetRef.current, PAGE_SIZE, opts)
      putRows(rows)
      offsetRef.current += rows.length
      setItems((prev) => prev.concat(rows.map(toCell)))
      if (rows.length < PAGE_SIZE) setHasMore(false)
      setError(null)
    } catch (e) {
      setError(String(e))
    } finally {
      loadingRef.current = false
    }
  }, [hasMore])

  const reset = useCallback(async () => {
    loadingRef.current = false
    offsetRef.current = 0
    setHasMore(true)
    setItems([])
    pendingUndoRef.current.clear()
    try {
      loadingRef.current = true
      const opts = queryRef.current
      const rows = await listImages(0, PAGE_SIZE, opts)
      putRows(rows)
      offsetRef.current = rows.length
      setItems(rows.map(toCell))
      if (rows.length < PAGE_SIZE) setHasMore(false)
      setError(null)
    } catch (e) {
      setError(String(e))
    } finally {
      loadingRef.current = false
    }
  }, [])

  // biome-ignore lint/correctness/useExhaustiveDependencies: reloadToken re-runs the query when membership changes under the same filter
  useEffect(() => {
    queryRef.current = {
      sort,
      direction,
      buckets,
      orientations,
      batchId,
      labels,
      collectionId,
      projectKey,
      query,
    }
    // The clears inside `reset` must land in the same commit that kicks off the
    // fetch: they also zero `offsetRef`/`loadingRef`, and deferring them past an
    // await lets a scroll-driven `loadNext` page against the old offset and
    // concat stale-order rows onto the new query.
    void reset()
  }, [
    reset,
    sort,
    direction,
    buckets,
    orientations,
    batchId,
    labels,
    collectionId,
    projectKey,
    query,
    reloadToken,
  ])

  useEffect(() => {
    let off: UnlistenFn | undefined
    ;(async () => {
      off = await listen("ingest://batch-done", () => {
        void reset()
        onLibraryChanged?.()
      })
    })()
    return () => off?.()
  }, [reset, onLibraryChanged])

  useEffect(() => {
    const offs: UnlistenFn[] = []
    let cancelled = false
    ;(async () => {
      offs.push(
        await listen<string[]>("library://images-trashed", (e) => {
          const ids = new Set(e.payload)
          if (ids.size === 0) return
          setItems((prev) => {
            const next: Cell[] = []
            prev.forEach((cell, idx) => {
              if (ids.has(cell.key)) {
                pendingUndoRef.current.set(cell.key, { cell, idx })
              } else {
                next.push(cell)
              }
            })
            offsetRef.current = Math.max(
              0,
              offsetRef.current - (prev.length - next.length)
            )
            return next
          })
          const sel = useSelection.getState().selection
          const kept = selectedImageIds(sel).filter((id) => !ids.has(id))
          if (kept.length !== selectedImageIds(sel).length) {
            useSelection.getState().selectImages(kept)
          }
          onLibraryChanged?.()
        })
      )
      offs.push(
        await listen<string[]>("library://images-restored", (e) => {
          const ids = e.payload
          if (ids.length === 0) return
          setItems((prev) => {
            const next = [...prev]
            const captured = ids
              .map((id) => pendingUndoRef.current.get(id))
              .filter((x): x is { cell: Cell; idx: number } => x !== undefined)
              .sort((a, b) => a.idx - b.idx)
            let added = 0
            captured.forEach(({ cell, idx }) => {
              if (next.some((c) => c.key === cell.key)) return
              next.splice(Math.min(idx, next.length), 0, cell)
              pendingUndoRef.current.delete(cell.key)
              added += 1
            })
            offsetRef.current += added
            return next
          })
          onLibraryChanged?.()
        })
      )
      if (cancelled) for (const o of offs) o()
    })()
    return () => {
      cancelled = true
      for (const o of offs) o()
    }
  }, [onLibraryChanged])

  // Plain click selects one, Cmd+click toggles, Shift+click takes the range
  // from the anchor in grid order.
  const clickCard = (e: React.MouseEvent, idx: number, id: string) => {
    const anchor = useSelection.getState().anchor
    const from = anchor ? items.findIndex((c) => c.key === anchor) : -1
    if (e.shiftKey && from >= 0) {
      const [lo, hi] = from < idx ? [from, idx] : [idx, from]
      selectImages(items.slice(lo, hi + 1).map((c) => c.key))
    } else if (e.metaKey || e.ctrlKey) {
      toggleImage(id)
    } else {
      selectImage(id)
    }
  }

  // Right-click and the menu act on the selection when the card is part of it,
  // otherwise on that card alone.
  const targetsFor = (id: string) =>
    selectedIds.includes(id) ? selectedIds : [id]

  return (
    <section className="relative flex min-h-0 flex-1 flex-col gap-2">
      {error && (
        <p className="text-destructive text-xs">
          Error: <span className="select-text">{error}</span>
        </p>
      )}
      <ThumbGrid
        items={items}
        cellSize={cellSize}
        emptyLabel="Nothing imported yet — drop a folder or click Add."
        onEndReached={loadNext}
        renderCell={(it, idx) => (
          <CardContextMenu
            mode="library"
            onOpen={() => {
              if (!selectedIds.includes(it.row.id)) selectImage(it.row.id)
            }}
            onMoveToTrash={() => void moveToTrash(targetsFor(it.row.id))}
            groups={groupMenus?.(targetsFor(it.row.id))}
          >
            <MarkedImageCard
              id={it.row.id}
              onMarked={onLibraryChanged}
              draggable
              hash={it.hash}
              filename={it.filename}
              status={it.status}
              selected={selectedIds.includes(it.key)}
              onActivate={() => {
                selectImage(it.row.id)
                setQuickviewIndex(idx)
              }}
              onSelect={(e) => clickCard(e, idx, it.row.id)}
            />
          </CardContextMenu>
        )}
      />
      {quickviewIndex !== null && (
        <Quickview
          items={items}
          index={quickviewIndex}
          onClose={() => setQuickviewIndex(null)}
          onIndexChange={setQuickviewIndex}
        />
      )}
      <SnackbarViewport />
    </section>
  )
}
