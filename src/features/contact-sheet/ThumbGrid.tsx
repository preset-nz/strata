import { useEffect, useRef, useState } from "react"
import { useVirtualizer } from "@tanstack/react-virtual"

const CELL = 130
const AFFORDANCE_ROW = 24
const CARD_H = CELL + AFFORDANCE_ROW
const GAP = 6

type Props<T extends { key: string }> = {
  items: T[]
  renderCell: (item: T) => React.ReactNode
  emptyLabel?: string
  onEndReached?: () => void
  endReachedThresholdRows?: number
}

export function ThumbGrid<T extends { key: string }>({
  items,
  renderCell,
  emptyLabel = "(none)",
  onEndReached,
  endReachedThresholdRows = 3,
}: Props<T>) {
  const parentRef = useRef<HTMLDivElement | null>(null)
  const [columns, setColumns] = useState(1)

  useEffect(() => {
    const el = parentRef.current
    if (!el) return
    const measure = () => {
      const width = el.clientWidth
      const cols = Math.max(1, Math.floor((width + GAP) / (CELL + GAP)))
      setColumns(cols)
    }
    measure()
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  const rowCount = Math.ceil(items.length / columns)
  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => CARD_H + GAP,
    overscan: 6,
  })

  const virtualItems = rowVirtualizer.getVirtualItems()
  const lastVirtualIndex = virtualItems[virtualItems.length - 1]?.index ?? -1
  useEffect(() => {
    if (!onEndReached || rowCount === 0) return
    if (lastVirtualIndex >= rowCount - endReachedThresholdRows) {
      onEndReached()
    }
  }, [lastVirtualIndex, rowCount, onEndReached, endReachedThresholdRows])

  return (
    <div
      ref={parentRef}
      className="max-h-[480px] min-h-[60px] overflow-y-auto rounded-sm border border-border"
    >
      {items.length === 0 ? (
        <div className="p-2 text-xs text-muted-foreground/70">{emptyLabel}</div>
      ) : (
        <div
          style={{ height: rowVirtualizer.getTotalSize() }}
          className="relative w-full"
        >
          {virtualItems.map((row) => {
            const start = row.index * columns
            const slice = items.slice(start, start + columns)
            return (
              <div
                key={row.key}
                style={{
                  transform: `translateY(${row.start}px)`,
                  gridTemplateColumns: `repeat(${columns}, ${CELL}px)`,
                  gap: GAP,
                  padding: GAP / 2,
                }}
                className="absolute top-0 left-0 box-border grid w-full"
              >
                {slice.map((item) => (
                  <div
                    key={item.key}
                    style={{ width: CELL, height: CARD_H }}
                  >
                    {renderCell(item)}
                  </div>
                ))}
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
