import { cn } from "@/lib/utils"

export const CARD_SIZE_MIN = 80
export const CARD_SIZE_MAX = 280
export const CARD_SIZE_DEFAULT = 130

type Props = {
  cardSize: number
  onCardSizeChange: (next: number) => void
}

export function StatusBar({ cardSize, onCardSizeChange }: Props) {
  return (
    <footer
      className={cn(
        "flex h-7 shrink-0 items-center gap-3 border-border border-t bg-card/40 px-3",
        "text-[10px] text-muted-foreground tracking-wide"
      )}
    >
      <div className="ml-auto flex items-center gap-2">
        <label
          className="font-heading font-semibold text-[11px] uppercase tracking-[0.08em]"
          htmlFor="card-size"
        >
          Size
        </label>
        <input
          id="card-size"
          type="range"
          min={CARD_SIZE_MIN}
          max={CARD_SIZE_MAX}
          step={10}
          value={cardSize}
          onChange={(e) => onCardSizeChange(Number(e.target.value))}
          className="h-1 w-32 cursor-pointer appearance-none rounded-full bg-border accent-foreground"
          aria-label="Card size"
        />
        <span className="w-8 text-right tabular-nums">{cardSize}px</span>
      </div>
    </footer>
  )
}
