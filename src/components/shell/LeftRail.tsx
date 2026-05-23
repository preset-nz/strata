import { Heart } from "@phosphor-icons/react"
import { cn } from "@/lib/utils"
import {
  VGA16_BUCKETS,
  VGA16_HEX,
  VGA16_LABEL,
  type Vga16Bucket,
} from "@/lib/vga16"
import {
  COLOUR_LABELS,
  COLOUR_SWATCH,
  type ColourLabel,
} from "@/components/image-card/colour-label"

export const SORT_KEYS = [
  "imported",
  "filename",
  "created",
  "updated",
  "colour",
] as const

export type SortKey = (typeof SORT_KEYS)[number]

const SORT_LABEL: Record<SortKey, string> = {
  imported: "Imported",
  filename: "Filename",
  created: "Created",
  updated: "Updated",
  colour: "Colour",
}

type LabelSelector = "favourite" | ColourLabel

type Props = {
  bucketCounts: Record<Vga16Bucket, number>
  selectedBuckets: Set<Vga16Bucket>
  onToggleBucket: (bucket: Vga16Bucket) => void
  labelCounts: Record<LabelSelector, number>
  selectedLabels: Set<LabelSelector>
  onToggleLabel: (label: LabelSelector) => void
  sort: SortKey
  onSortChange: (next: SortKey) => void
}

export function LeftRail({
  bucketCounts,
  selectedBuckets,
  onToggleBucket,
  labelCounts,
  selectedLabels,
  onToggleLabel,
  sort,
  onSortChange,
}: Props) {
  return (
    <aside className="flex w-[220px] shrink-0 flex-col gap-4 overflow-y-auto border-r border-border bg-card/40 p-3 text-xs">
      <section className="flex flex-col gap-1.5">
        <SectionHeading>Sort</SectionHeading>
        <select
          value={sort}
          onChange={(e) => onSortChange(e.target.value as SortKey)}
          className={cn(
            "h-7 rounded-sm border border-border bg-background px-2 text-xs",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          )}
        >
          {SORT_KEYS.map((k) => (
            <option key={k} value={k}>
              {SORT_LABEL[k]}
            </option>
          ))}
        </select>
      </section>

      <section className="flex flex-col gap-1">
        <SectionHeading>Content colour</SectionHeading>
        <ul className="flex flex-col">
          {VGA16_BUCKETS.map((b) => {
            const count = bucketCounts[b] ?? 0
            const selected = selectedBuckets.has(b)
            const empty = count === 0 && !selected
            return (
              <li key={b}>
                <button
                  type="button"
                  aria-pressed={selected}
                  disabled={empty}
                  onClick={() => onToggleBucket(b)}
                  className={cn(
                    "group flex w-full items-center gap-2 rounded-sm px-1 py-0.5 text-left transition-colors",
                    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                    selected
                      ? "bg-muted text-foreground"
                      : empty
                        ? "text-muted-foreground/40"
                        : "text-foreground hover:bg-muted/60",
                  )}
                >
                  <span
                    className={cn(
                      "inline-flex size-3 shrink-0 items-center justify-center rounded-[2px] border",
                      selected
                        ? "border-foreground/60 bg-foreground/10"
                        : "border-border",
                    )}
                  >
                    {selected && (
                      <span className="size-1.5 rounded-[1px] bg-foreground" />
                    )}
                  </span>
                  <span
                    className="size-3 shrink-0 rounded-[2px] border border-border/70"
                    style={{ backgroundColor: VGA16_HEX[b] }}
                  />
                  <span className="flex-1 truncate">{VGA16_LABEL[b]}</span>
                  <span className="tabular-nums text-[10px] text-muted-foreground">
                    {count.toLocaleString()}
                  </span>
                </button>
              </li>
            )
          })}
        </ul>
      </section>

      <section className="flex flex-col gap-1">
        <SectionHeading>Label</SectionHeading>
        <ul className="flex flex-col">
          <LabelRow
            id="favourite"
            label="Favourited"
            count={labelCounts.favourite ?? 0}
            selected={selectedLabels.has("favourite")}
            onClick={() => onToggleLabel("favourite")}
            swatch={
              <Heart
                weight="fill"
                className="size-3 shrink-0 text-red-500"
              />
            }
          />
          {COLOUR_LABELS.map((c) => (
            <LabelRow
              key={c}
              id={c}
              label={c.charAt(0).toUpperCase() + c.slice(1)}
              count={labelCounts[c] ?? 0}
              selected={selectedLabels.has(c)}
              onClick={() => onToggleLabel(c)}
              swatch={
                <span
                  className={cn(
                    "inline-block size-3 shrink-0 rounded-full",
                    COLOUR_SWATCH[c],
                  )}
                />
              }
            />
          ))}
        </ul>
      </section>
    </aside>
  )
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-[10px] font-medium tracking-wide text-muted-foreground uppercase">
      {children}
    </h2>
  )
}

function LabelRow({
  label,
  count,
  selected,
  onClick,
  swatch,
}: {
  id: LabelSelector
  label: string
  count: number
  selected: boolean
  onClick: () => void
  swatch: React.ReactNode
}) {
  const empty = count === 0 && !selected
  return (
    <li>
      <button
        type="button"
        aria-pressed={selected}
        disabled={empty}
        onClick={onClick}
        className={cn(
          "flex w-full items-center gap-2 rounded-sm px-1 py-0.5 text-left transition-colors",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          selected
            ? "bg-muted text-foreground"
            : empty
              ? "text-muted-foreground/40"
              : "text-foreground hover:bg-muted/60",
        )}
      >
        {swatch}
        <span className="flex-1 truncate">{label}</span>
        <span className="tabular-nums text-[10px] text-muted-foreground">
          {count.toLocaleString()}
        </span>
      </button>
    </li>
  )
}
