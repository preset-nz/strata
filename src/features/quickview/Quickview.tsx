import { useEffect } from "react"
import { X, CaretLeft, CaretRight } from "@phosphor-icons/react"
import { cn } from "@/lib/utils"
import { registerOverlay } from "@/lib/overlay"

type Item = {
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
        className="flex h-10 shrink-0 items-center gap-3 border-b border-border/60 px-3"
        onClick={(e) => e.stopPropagation()}
      >
        <span className="select-text truncate text-xs text-muted-foreground">
          {item.filename}
        </span>
        <span className="ml-auto font-heading text-[10px] tracking-[0.08em] uppercase text-muted-foreground tabular-nums">
          {index + 1} / {items.length}
        </span>
        <button
          type="button"
          aria-label="Close quickview"
          onClick={onClose}
          className={cn(
            "inline-flex size-7 items-center justify-center rounded-sm text-muted-foreground",
            "hover:bg-muted hover:text-foreground",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
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
          <img
            src={`thumb://${item.hash}/1024.jpg`}
            alt={item.filename}
            onClick={(e) => e.stopPropagation()}
            className="max-h-full max-w-full object-contain shadow-2xl"
          />
        ) : (
          <p className="text-sm text-muted-foreground">no thumb</p>
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
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
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
