import { CaretLeft, CaretRight, Crosshair, X } from "@phosphor-icons/react"
import { useEffect } from "react"
import { registerOverlay } from "@/lib/overlay"
import { cn } from "@/lib/utils"
import { MarkerOverlay } from "./MarkerOverlay"
import {
  MARKER_COUNTS,
  paletteMarkers,
  usePaletteMarkers,
} from "./palette-markers"

type Item = {
  /** The image id. */
  key: string
  hash: string
  filename: string
  status: string
}

type Props = {
  items: Item[]
  index: number
  onClose: () => void
  onIndexChange: (next: number) => void
}

export function Quickview({ items, index, onClose, onIndexChange }: Props) {
  const item = items[index]
  const markers = usePaletteMarkers()

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault()
        onClose()
      } else if (e.key === "ArrowLeft" && index > 0) {
        e.preventDefault()
        onIndexChange(index - 1)
      } else if (e.key === "ArrowRight" && index < items.length - 1) {
        e.preventDefault()
        onIndexChange(index + 1)
      } else if (e.key === " ") {
        e.preventDefault()
        onClose()
      }
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [index, items.length, onClose, onIndexChange])

  // Quickview owns the screen while open; panel shortcuts stand down.
  useEffect(() => registerOverlay(), [])

  if (!item) return null

  const canPrev = index > 0
  const canNext = index < items.length - 1

  return (
    <div
      role="dialog"
      aria-modal="true"
      onClick={onClose}
      className="fixed inset-0 z-50 flex flex-col bg-background/95 backdrop-blur-sm"
    >
      <header
        className="flex h-10 shrink-0 items-center gap-3 border-border/60 border-b px-3"
        onClick={(e) => e.stopPropagation()}
      >
        <span className="select-text truncate text-muted-foreground text-xs">
          {item.filename}
        </span>
        <div className="ml-auto flex items-center gap-1">
          <button
            type="button"
            aria-pressed={markers.on}
            onClick={paletteMarkers.toggle}
            title="Palette markers  ⇧⌘M"
            className={cn(
              "inline-flex h-7 items-center gap-1 rounded-sm px-1.5 text-xs",
              markers.on
                ? "bg-muted text-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground"
            )}
          >
            <Crosshair weight="bold" />
          </button>
          {markers.on &&
            MARKER_COUNTS.map((n) => (
              <button
                key={n}
                type="button"
                aria-pressed={markers.count === n}
                onClick={() => paletteMarkers.setCount(n)}
                className={cn(
                  "h-7 rounded-sm px-1.5 text-xs tabular-nums",
                  markers.count === n
                    ? "bg-muted text-foreground"
                    : "text-muted-foreground hover:bg-muted"
                )}
              >
                {n}
              </button>
            ))}
        </div>
        <span className="font-heading text-[10px] text-muted-foreground uppercase tabular-nums tracking-[0.08em]">
          {index + 1} / {items.length}
        </span>
        <button
          type="button"
          aria-label="Close quickview"
          onClick={onClose}
          className={cn(
            "inline-flex size-7 items-center justify-center rounded-sm text-muted-foreground",
            "hover:bg-muted hover:text-foreground",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          )}
        >
          <X weight="bold" />
        </button>
      </header>

      <div
        className="relative flex min-h-0 flex-1 items-center justify-center p-6"
        onClick={onClose}
      >
        {item.status === "ready" ? (
          // The wrapper shrinks to the image, so markers placed in fractions
          // of it land on the picture, not on letterbox space.
          <div
            className="relative max-h-full max-w-full"
            onClick={(e) => e.stopPropagation()}
          >
            <img
              src={`thumb://${item.hash}/1024.jpg`}
              alt={item.filename}
              className="block max-h-[calc(100svh-7rem)] max-w-full shadow-2xl"
            />
            {markers.on && (
              <MarkerOverlay imageId={item.key} count={markers.count} />
            )}
          </div>
        ) : (
          <p className="text-muted-foreground text-sm">no thumb</p>
        )}

        {canPrev && (
          <NavButton
            side="left"
            onClick={(e) => {
              e.stopPropagation()
              onIndexChange(index - 1)
            }}
          />
        )}
        {canNext && (
          <NavButton
            side="right"
            onClick={(e) => {
              e.stopPropagation()
              onIndexChange(index + 1)
            }}
          />
        )}
      </div>
    </div>
  )
}

function NavButton({
  side,
  onClick,
}: {
  side: "left" | "right"
  onClick: (e: React.MouseEvent) => void
}) {
  return (
    <button
      type="button"
      aria-label={side === "left" ? "Previous" : "Next"}
      onClick={onClick}
      className={cn(
        "absolute top-1/2 -translate-y-1/2",
        side === "left" ? "left-4" : "right-4",
        "inline-flex size-10 items-center justify-center rounded-sm",
        "bg-card/60 text-muted-foreground backdrop-blur-sm",
        "hover:bg-card hover:text-foreground",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      )}
    >
      {side === "left" ? (
        <CaretLeft weight="bold" />
      ) : (
        <CaretRight weight="bold" />
      )}
    </button>
  )
}
