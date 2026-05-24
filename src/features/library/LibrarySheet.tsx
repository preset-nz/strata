import { useCallback, useEffect, useRef, useState } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { ImageCard } from "@/components/image-card"
import { CardContextMenu } from "@/components/image-card/CardContextMenu"
import { Quickview } from "@/features/quickview/Quickview"
import { ThumbGrid } from "../contact-sheet/ThumbGrid"
import type { ImportedRow } from "../contact-sheet/api"
import { listImages, type SortDirection } from "./api"
import { useMoveToTrash } from "./use-move-to-trash"
import type { Vga16Bucket } from "@/lib/vga16"
import type { SortKey } from "@/components/shell/LeftRail"
import { useSelection } from "@/stores/selection"
import { SnackbarViewport } from "@/components/ui/snackbar"

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
    filename: row.original_filename,
    status: row.thumbnails_status,
    row,
  }
}

type Props = {
  sort: SortKey
  direction: SortDirection
  buckets: Vga16Bucket[]
  batchId?: string | null
  cellSize?: number
  onLibraryChanged?: () => void
}

export function LibrarySheet({
  sort,
  direction,
  buckets,
  batchId,
  cellSize,
  onLibraryChanged,
}: Props) {
  const [items, setItems] = useState<Cell[]>([])
  const [hasMore, setHasMore] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [quickviewIndex, setQuickviewIndex] = useState<number | null>(null)
  const loadingRef = useRef(false)
  const offsetRef = useRef(0)
  const queryRef = useRef({ sort, direction, buckets, batchId })
  queryRef.current = { sort, direction, buckets, batchId }
  const pendingUndoRef = useRef<Map<string, { cell: Cell; idx: number }>>(
    new Map(),
  )
  const { selection, selectImage } = useSelection()
  const selectedImageId =
    selection.kind === "image" ? selection.id : null
  const moveToTrash = useMoveToTrash()

  const loadNext = useCallback(async () => {
    if (loadingRef.current || !hasMore) return
    loadingRef.current = true
    try {
      const { sort, direction, buckets, batchId } = queryRef.current
      const opts = { sort, direction, buckets, batchId }
      const rows = await listImages(offsetRef.current, PAGE_SIZE, opts)
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
      const { sort, direction, buckets, batchId } = queryRef.current
      const opts = { sort, direction, buckets, batchId }
      const rows = await listImages(0, PAGE_SIZE, opts)
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

  useEffect(() => {
    void reset()
  }, [reset, sort, direction, buckets, batchId])

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
            offsetRef.current = Math.max(0, offsetRef.current - (prev.length - next.length))
            return next
          })
          const sel = useSelection.getState().selection
          if (sel.kind === "image" && ids.has(sel.id)) {
            useSelection.getState().clear()
          }
          onLibraryChanged?.()
        }),
      )
      offs.push(
        await listen<string[]>("library://images-restored", (e) => {
          const ids = e.payload
          if (ids.length === 0) return
          setItems((prev) => {
            const next = [...prev]
            const captured = ids
              .map((id) => pendingUndoRef.current.get(id))
              .filter(
                (x): x is { cell: Cell; idx: number } => x !== undefined,
              )
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
        }),
      )
      if (cancelled) offs.forEach((o) => o())
    })()
    return () => {
      cancelled = true
      offs.forEach((o) => o())
    }
  }, [onLibraryChanged])

  return (
    <section className="relative flex min-h-0 flex-1 flex-col gap-2">
      {error && (
        <p className="text-xs text-destructive">
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
            onMoveToTrash={() => void moveToTrash([it.row.id])}
          >
            <ImageCard
              id={it.row.id}
              draggable
              hash={it.hash}
              filename={it.filename}
              status={it.status}
              selected={selectedImageId === it.key}
              onActivate={() => {
                selectImage(it.row.id)
                setQuickviewIndex(idx)
              }}
              onSelect={() => selectImage(it.row.id)}
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
