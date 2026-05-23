import { cn } from "@/lib/utils"
import { Thumbnail } from "@/features/contact-sheet/Thumbnail"
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
}: Props) {
  return (
    <div
      className={cn(
        "group/card relative flex h-full w-full flex-col",
        "rounded-sm border bg-card p-1 shadow-sm transition-shadow",
        selected
          ? "border-foreground/60 shadow"
          : "border-border/70 hover:shadow",
      )}
    >
      <div className="min-h-0 flex-1 overflow-hidden bg-muted/30">
        <Thumbnail hash={hash} filename={filename} status={status} />
      </div>
      <div className="mt-1 flex h-4 items-center justify-between px-0.5">
        <FavouriteToggle
          isFavourite={isFavourite}
          onToggle={() => onFavouriteToggle?.()}
        />
        <ColourLabelSwatch
          value={colourLabel}
          onChange={(next) => onColourLabelChange?.(next)}
        />
      </div>
    </div>
  )
}
