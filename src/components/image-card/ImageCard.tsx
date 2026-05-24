import { cn } from "@/lib/utils"
import { Thumbnail } from "@/features/contact-sheet/Thumbnail"
import { useHardPress } from "@/lib/use-hard-press"
import { FavouriteToggle } from "./FavouriteToggle"
import { ColourLabelSwatch } from "./ColourLabelSwatch"
import type { ColourLabel } from "./colour-label"
import { useDraggableCard } from "./use-draggable-card"

type Props = {
  id?: string
  hash: string
  filename: string
  status: "ready" | "missing" | "failed" | string
  isFavourite?: boolean
  colourLabel?: ColourLabel | null
  selected?: boolean
  draggable?: boolean
  badge?: React.ReactNode
  onFavouriteToggle?: () => void
  onColourLabelChange?: (next: ColourLabel | null) => void
  onActivate?: () => void
  onSelect?: () => void
}

export function ImageCard({
  id,
  hash,
  filename,
  status,
  isFavourite = false,
  colourLabel = null,
  selected = false,
  draggable = false,
  badge,
  onFavouriteToggle,
  onColourLabelChange,
  onActivate,
  onSelect,
}: Props) {
  const pressRef = useHardPress<HTMLDivElement>(onActivate)
  const dragProps = useDraggableCard({ id: id ?? "" })
  const dragHandlers = draggable && id ? dragProps : null
  return (
    <div
      ref={pressRef}
      {...(dragHandlers ?? {})}
      onClick={(e) => {
        const target = e.target as Element | null
        if (target?.closest("button, [role='button'], input, select")) return
        onSelect?.()
      }}
      className={cn(
        "group/card relative flex h-full w-full flex-col cursor-pointer",
        "rounded-sm border bg-card p-1 shadow-sm",
        "transition-[box-shadow,border-color,background-color] duration-150 ease-out",
        selected
          ? "border-foreground/60 shadow"
          : "border-border/70 hover:bg-muted hover:shadow",
      )}
    >
      <div className="relative aspect-square w-full overflow-hidden bg-muted/30">
        <Thumbnail hash={hash} filename={filename} status={status} />
        {badge && (
          <div className="pointer-events-none absolute top-1 right-1">
            {badge}
          </div>
        )}
      </div>
      <div className="mt-1 flex h-4 items-center justify-between px-0.5">
        <ColourLabelSwatch
          value={colourLabel}
          onChange={(next) => onColourLabelChange?.(next)}
        />
        <FavouriteToggle
          isFavourite={isFavourite}
          onToggle={() => onFavouriteToggle?.()}
        />
      </div>
    </div>
  )
}
