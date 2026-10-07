import { type ReactNode, useCallback, useRef } from "react"
import {
  COLLAPSED_WIDTH,
  clampWidth,
  type PanelSide,
} from "@/components/shell/panel-geometry"
import { cn } from "@/lib/utils"

type Props = {
  side: PanelSide
  // Shown along the spine when collapsed, so the panel never reads as absent.
  title: string
  width: number
  onWidthChange: (width: number) => void
  collapsed: boolean
  onExpand: () => void
  defaultWidth: number
  minWidth: number
  maxWidth: number
  children: ReactNode
}

export function SidePanel({
  side,
  title,
  width,
  onWidthChange,
  collapsed,
  onExpand,
  defaultWidth,
  minWidth,
  maxWidth,
  children,
}: Props) {
  // Drag origin, captured on pointerdown. Width is derived from the origin
  // rather than accumulated per-move, so a dropped frame can't drift the panel.
  const drag = useRef<{ startX: number; startWidth: number } | null>(null)

  const onPointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (e.button !== 0) return
      e.preventDefault()
      drag.current = { startX: e.clientX, startWidth: width }
      e.currentTarget.setPointerCapture(e.pointerId)
    },
    [width]
  )

  const onPointerMove = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      const d = drag.current
      if (!d) return
      const dx = e.clientX - d.startX
      // The handle is on the panel's inner edge: dragging right grows a left
      // panel and shrinks a right one.
      const raw = side === "left" ? d.startWidth + dx : d.startWidth - dx
      onWidthChange(clampWidth(raw, minWidth, maxWidth))
    },
    [side, onWidthChange, minWidth, maxWidth]
  )

  const endDrag = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (!drag.current) return
    drag.current = null
    if (e.currentTarget.hasPointerCapture(e.pointerId)) {
      e.currentTarget.releasePointerCapture(e.pointerId)
    }
  }, [])

  const onDoubleClick = useCallback(() => {
    onWidthChange(clampWidth(defaultWidth, minWidth, maxWidth))
  }, [onWidthChange, defaultWidth, minWidth, maxWidth])

  const edge = side === "left" ? "border-r" : "border-l"

  if (collapsed) {
    return (
      <button
        type="button"
        onClick={onExpand}
        aria-label={`Expand ${title.toLowerCase()} panel`}
        title={`Expand ${title.toLowerCase()} panel`}
        style={{ width: COLLAPSED_WIDTH }}
        className={cn(
          "flex h-full shrink-0 cursor-pointer items-center justify-center",
          "bg-card/40 text-muted-foreground hover:bg-accent hover:text-foreground",
          "border-border",
          edge
        )}
      >
        <span className="rotate-180 font-semibold text-[10px] uppercase tracking-wider [writing-mode:vertical-rl]">
          {title}
        </span>
      </button>
    )
  }

  return (
    <div
      style={{ width }}
      className={cn(
        "relative flex h-full shrink-0 flex-col border-border bg-card/40",
        edge
      )}
    >
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        {children}
      </div>
      <hr
        aria-orientation="vertical"
        aria-label={`Resize ${title.toLowerCase()} panel`}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onDoubleClick={onDoubleClick}
        // Straddles the panel's inner edge: a 1px border is not a hit target.
        className={cn(
          "absolute inset-y-0 z-10 m-0 h-auto w-1.5 cursor-col-resize touch-none border-0",
          "hover:bg-primary/40 active:bg-primary/60",
          side === "left" ? "-right-px" : "-left-px"
        )}
      />
    </div>
  )
}
