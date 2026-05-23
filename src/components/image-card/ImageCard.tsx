import { cn } from "@/lib/utils"
import { Thumbnail } from "@/features/contact-sheet/Thumbnail"
import { useHardPress } from "@/lib/use-hard-press"
import { FavouriteToggle } from "./FavouriteToggle"
import { ColourLabelSwatch } from "./ColourLabelSwatch"
import type { ColourLabel } from "./colour-label"

type Props = {
  hash: string
  filename: string
  status: "ready" | "missing" | "failed" | string
  isFavourite?: boolean
  colourLabel?: ColourLabel | null
  selected?: boolean
  onFavouriteToggle?: () => void
  onColourLabelChange?: (next: ColourLabel | null) => void
  onActivate?: () => void
}

export function ImageCard({
  hash,
  filename,
  status,
  isFavourite = false,
  colourLabel = null,
  selected = false,
  onFavouriteToggle,
  onColourLabelChange,
  onActivate,
}: Props) {
  const pressRef = useHardPress<HTMLDivElement>(onActivate)
  return (
    <div
      ref={pressRef}
      className={cn(
        "group/card relative flex h-full w-full flex-col",
        "rounded-sm border bg-card p-1 shadow-sm",
        "transition-[box-shadow,border-color,background-color] duration-150 ease-out",
        selected
          ? "border-foreground/60 shadow"
          : "border-border/70 hover:border-foreground/40 hover:bg-accent/40 hover:shadow",
      )}
    >
      <div className="aspect-square w-full overflow-hidden bg-muted/30">
        <Thumbnail hash={hash} filename={filename} status={status} />
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
