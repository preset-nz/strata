import { Heart } from "@phosphor-icons/react"
import { cn } from "@/lib/utils"

type Props = {
  isFavourite: boolean
  onToggle: () => void
}

export function FavouriteToggle({ isFavourite, onToggle }: Props) {
  return (
    <button
      type="button"
      aria-label={isFavourite ? "Unfavourite" : "Favourite"}
      aria-pressed={isFavourite}
      onClick={(e) => {
        e.stopPropagation()
        onToggle()
      }}
      className={cn(
        "inline-flex size-4 items-center justify-center rounded-sm outline-none transition-opacity",
        "focus-visible:ring-1 focus-visible:ring-ring",
        isFavourite
          ? "text-red-500 opacity-100"
          : "text-muted-foreground/60 opacity-0 group-hover/card:opacity-100"
      )}
    >
      <Heart weight={isFavourite ? "fill" : "regular"} className="size-3.5" />
    </button>
  )
}
