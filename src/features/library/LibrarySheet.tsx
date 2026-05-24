import { useCallback, useEffect, useRef, useState } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { ImageCard } from "@/components/image-card"
import { CardContextMenu } from "@/components/image-card/CardContextMenu"
import { Quickview } from "@/features/quickview/Quickview"
import { ThumbGrid } from "../contact-sheet/ThumbGrid"
import type { ImportedRow } from "../contact-sheet/api"
import { deleteImages, listImages, restoreImages, type SortDirection } from "./api"
import type { Vga16Bucket } from "@/lib/vga16"
import type { SortKey } from "@/components/shell/LeftRail"
import { useSelection } from "@/stores/selection"
import { SnackbarViewport, useSnackbar } from "@/components/ui/snackbar"

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
  const { selection, selectImage, clear: clearSelection } = useSelection()
  const selectedImageId =
    selection.kind === "image" ? selection.id : null
  const { show: showSnackbar } = useSnackbar()

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

  const moveToTrash = useCallback(
    async (id: string) => {
      const idx = items.findIndex((c) => c.key === id)
      if (idx === -1) return
      const removed = items[idx]

      setItems((prev) => prev.filter((c) => c.key !== id))
      offsetRef.current = Math.max(0, offsetRef.current - 1)
      if (selectedImageId === id) clearSelection()

      try {
        await deleteImages([id])
        onLibraryChanged?.()
        showSnackbar({
          message: "Moved to Trash",
          action: {
            label: "Undo",
            onClick: () => {
              void (async () => {
                try {
                  await restoreImages([id])
                  setItems((prev) => {
                    if (prev.some((c) => c.key === id)) return prev
                    const next = [...prev]
                    next.splice(Math.min(idx, next.length), 0, removed)
                    return next
                  })
                  offsetRef.current += 1
                  onLibraryChanged?.()
                } catch (e) {
                  setError(String(e))
                }
              })()
            },
          },
        })
      } catch (e) {
        setItems((prev) => {
          const next = [...prev]
          next.splice(Math.min(idx, next.length), 0, removed)
          return next
        })
        setError(String(e))
      }
    },
    [items, selectedImageId, clearSelection, onLibraryChanged, showSnackbar],
  )

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
            onMoveToTrash={() => void moveToTrash(it.row.id)}
          >
            <ImageCard
              hash={it.hash}
              filename={it.filename}
              status={it.status}
              selected={selectedImageId === it.key}
              onActivate={() => setQuickviewIndex(idx)}
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
