import { Crosshair } from "@phosphor-icons/react"
import { QuickView } from "@preset.nz/ux-kit"
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

// Strata's Quickview: the shell's frame and keys, with the thumbnail, the
// palette markers and the overlay registration (panel shortcuts stand down)
// that belong to this app.
export function Quickview({ items, index, onClose, onIndexChange }: Props) {
  const markers = usePaletteMarkers()
  return (
    <QuickView
      items={items}
      index={index}
      onClose={onClose}
      onIndexChange={onIndexChange}
      onOverlay={registerOverlay}
      title={(item) => item.filename}
      tools={
        <>
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
        </>
      }
      renderItem={(item) =>
        item.status === "ready" ? (
          // The wrapper shrinks to the image, so markers placed in fractions
          // of it land on the picture, not on letterbox space.
          <div className="relative max-h-full max-w-full">
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
        )
      }
    />
  )
}
