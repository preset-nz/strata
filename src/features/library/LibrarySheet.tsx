import { useCallback, useEffect, useRef, useState } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { ImageCard } from "@/components/image-card"
import { Quickview } from "@/features/quickview/Quickview"
import { ThumbGrid } from "../contact-sheet/ThumbGrid"
import type { ImportedRow } from "../contact-sheet/api"
import { listImages } from "./api"
import type { Vga16Bucket } from "@/lib/vga16"
import type { SortKey } from "@/components/shell/LeftRail"
import { useSelection } from "@/stores/selection"

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
  buckets: Vga16Bucket[]
  batchId?: string | null
  cellSize?: number
  onLibraryChanged?: () => void
}

export function LibrarySheet({
  sort,
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
  const queryRef = useRef({ sort, buckets, batchId })
  queryRef.current = { sort, buckets, batchId }
  const { selection, selectImage } = useSelection()
  const selectedImageId =
    selection.kind === "image" ? selection.id : null

  const loadNext = useCallback(async () => {
    if (loadingRef.current || !hasMore) return
    loadingRef.current = true
    try {
      const { sort, buckets, batchId } = queryRef.current
      const opts = { sort, buckets, batchId }
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
      const { sort, buckets, batchId } = queryRef.current
      const opts = { sort, buckets, batchId }
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
  }, [reset, sort, buckets, batchId])

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

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-2">
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
          <ImageCard
            hash={it.hash}
            filename={it.filename}
            status={it.status}
            selected={selectedImageId === it.key}
            onActivate={() => setQuickviewIndex(idx)}
            onSelect={() => selectImage(it.row)}
          />
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
    </section>
  )
}
