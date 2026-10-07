import { Popover } from "@base-ui/react/popover"
import { cn } from "@/lib/utils"
import { COLOUR_LABELS, COLOUR_SWATCH, type ColourLabel } from "./colour-label"

type Props = {
  value: ColourLabel | null
  onChange: (next: ColourLabel | null) => void
}

export function ColourLabelSwatch({ value, onChange }: Props) {
  return (
    <Popover.Root>
      <Popover.Trigger
        aria-label={value ? `Colour label: ${value}` : "Set colour label"}
        onClick={(e) => e.stopPropagation()}
        className={cn(
          "inline-flex size-4 items-center justify-center rounded-full outline-none transition-opacity",
          "focus-visible:ring-1 focus-visible:ring-ring",
          value ? "opacity-100" : "opacity-0 group-hover/card:opacity-100"
        )}
      >
        <span
          className={cn(
            "block size-3 rounded-full",
            value
              ? COLOUR_SWATCH[value]
              : "border border-muted-foreground/60 border-dashed"
          )}
        />
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner sideOffset={6} align="center">
          <Popover.Popup
            onClick={(e) => e.stopPropagation()}
            className="z-50 flex items-center gap-1 rounded-sm border border-border bg-popover p-1.5 shadow-md outline-none"
          >
            {COLOUR_LABELS.map((c) => (
              <button
                key={c}
                type="button"
                aria-label={c}
                aria-pressed={value === c}
                onClick={() => onChange(c)}
                className={cn(
                  "size-4 rounded-full outline-none transition-transform hover:scale-110",
                  "focus-visible:ring-1 focus-visible:ring-ring",
                  COLOUR_SWATCH[c],
                  value === c && "ring-2 ring-foreground/40"
                )}
              />
            ))}
            <button
              type="button"
              aria-label="clear"
              onClick={() => onChange(null)}
              className={cn(
                "ml-1 size-4 rounded-full border border-muted-foreground/60 border-dashed outline-none transition-transform hover:scale-110",
                "focus-visible:ring-1 focus-visible:ring-ring"
              )}
            />
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
