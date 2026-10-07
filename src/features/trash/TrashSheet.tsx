import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { useCallback, useEffect, useRef, useState } from "react"
import { ImageCard } from "@/components/image-card"
import { CardContextMenu } from "@/components/image-card/CardContextMenu"
import { SnackbarViewport } from "@/components/ui/snackbar"
import { cn } from "@/lib/utils"
import { useSelection } from "@/stores/selection"
import type { ImportedRow } from "../contact-sheet/api"
import { ThumbGrid } from "../contact-sheet/ThumbGrid"
import {
  listImages,
  purgeImages,
  restoreImages,
  type SortDirection,
} from "../library/api"

type Cell = {
  key: string
  hash: string
  filename: string
  status: string
  deletedAt: string | null
  row: ImportedRow
}

const PAGE_SIZE = 100
const RETENTION_DAYS = 30

function toCell(row: ImportedRow): Cell {
  return {
    key: row.id,
    hash: row.content_hash,
    filename: row.title,
    status: row.thumbnails_status,
    deletedAt: row.deleted_at,
    row,
  }
}

function daysRemaining(deletedAt: string | null): number {
  if (!deletedAt) return RETENTION_DAYS
  const ms = Date.now() - new Date(deletedAt).getTime()
  return Math.max(0, Math.ceil(RETENTION_DAYS - ms / 86_400_000))
}

/** Local-day key for grouping. Returns "" for null/invalid timestamps so they
 * cluster together at the end of a list and don't churn the badge. */
function dayKey(deletedAt: string | null): string {
  if (!deletedAt) return ""
  const d = new Date(deletedAt)
  if (Number.isNaN(d.getTime())) return ""
  return d.toDateString()
}

type Props = {
  direction: SortDirection
  cellSize?: number
  onLibraryChanged?: () => void
}

export function TrashSheet({ direction, cellSize, onLibraryChanged }: Props) {
  const [items, setItems] = useState<Cell[]>([])
  const [hasMore, setHasMore] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const loadingRef = useRef(false)
  const offsetRef = useRef(0)
  const queryRef = useRef({ direction })
  const { selection, selectImage } = useSelection()
  const selectedImageId = selection.kind === "image" ? selection.id : null

  const loadNext = useCallback(async () => {
    if (loadingRef.current || !hasMore) return
    loadingRef.current = true
    try {
      const { direction } = queryRef.current
      const rows = await listImages(offsetRef.current, PAGE_SIZE, {
        sort: "deleted",
        direction,
        onlyDeleted: true,
      })
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
      const { direction } = queryRef.current
      const rows = await listImages(0, PAGE_SIZE, {
        sort: "deleted",
        direction,
        onlyDeleted: true,
      })
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
    queryRef.current = { direction }
    // See LibrarySheet: the clears in `reset` also zero `offsetRef`/`loadingRef`
    // and must commit alongside the fetch kickoff, or a scroll-driven `loadNext`
    // races the offset.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void reset()
  }, [reset, direction])

  useEffect(() => {
    const offs: UnlistenFn[] = []
    let cancelled = false
    const removeIds = (ids: string[]) => {
      if (ids.length === 0) return
      const idSet = new Set(ids)
      setItems((prev) => {
        const next = prev.filter((c) => !idSet.has(c.key))
        offsetRef.current = Math.max(
          0,
          offsetRef.current - (prev.length - next.length)
        )
        return next
      })
      const sel = useSelection.getState().selection
      if (sel.kind === "image" && idSet.has(sel.id)) {
        useSelection.getState().clear()
      }
      onLibraryChanged?.()
    }
    ;(async () => {
      // Restored items leave the Trash collection.
      offs.push(
        await listen<string[]>("library://images-restored", (e) => {
          removeIds(e.payload)
        })
      )
      // Hard-deleted items leave the Trash collection (and the catalog).
      offs.push(
        await listen<string[]>("library://images-purged", (e) => {
          removeIds(e.payload)
        })
      )
      // A fresh soft-delete from elsewhere bumps the count and should appear
      // here on next entry — refresh so it lands in the current view too.
      offs.push(
        await listen<string[]>("library://images-trashed", () => {
          void reset()
        })
      )
      if (cancelled) offs.forEach((o) => o())
    })()
    return () => {
      cancelled = true
      offs.forEach((o) => o())
    }
  }, [reset, onLibraryChanged])

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
        emptyLabel="Trash is empty."
        onEndReached={loadNext}
        renderCell={(it, idx) => {
          const prev = idx > 0 ? items[idx - 1] : null
          const showBadge =
            !prev || dayKey(prev.deletedAt) !== dayKey(it.deletedAt)
          const days = daysRemaining(it.deletedAt)
          return (
            <CardContextMenu
              mode="trash"
              onRestore={() => void restoreImages([it.row.id])}
              onDeletePermanently={() => void purgeImages([it.row.id])}
            >
              <ImageCard
                id={it.row.id}
                hash={it.hash}
                filename={it.filename}
                status={it.status}
                selected={selectedImageId === it.key}
                onSelect={() => selectImage(it.row.id)}
                badge={showBadge ? <DaysBadge days={days} /> : undefined}
              />
            </CardContextMenu>
          )
        }}
      />
      <SnackbarViewport />
    </section>
  )
}

function DaysBadge({ days }: { days: number }) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-xs px-1 py-0.5 font-medium text-[9px] tabular-nums shadow-sm",
        "bg-background/85 backdrop-blur-sm",
        days <= 3
          ? "text-destructive ring-1 ring-destructive/40"
          : "text-foreground/80"
      )}
      title={`${days} ${days === 1 ? "day" : "days"} until permanent deletion`}
    >
      {days}d
    </span>
  )
}
