import { useState } from "react"
import { ArrowDown, ArrowUp, CaretRight, Heart, Trash } from "@phosphor-icons/react"
import { cn } from "@/lib/utils"
import { readStrataImagePayload } from "@/components/image-card/use-draggable-card"
import { usePersistedState } from "@/lib/use-persisted-state"
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
import type { BatchSummary } from "@/features/library/api"

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

export type SortDirection = "asc" | "desc"

export const DEFAULT_DIRECTION: Record<SortKey, SortDirection> = {
  imported: "desc",
  filename: "asc",
  created: "desc",
  updated: "desc",
  colour: "asc",
}

type SectionId = "sort" | "content_colour" | "label" | "imports" | "system"

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
  direction: SortDirection
  onDirectionToggle: () => void
  batches: BatchSummary[]
  selectedBatchId: string | null
  onSelectBatch: (id: string | null) => void
  trashCount: number
  onTrashDrop: (ids: string[]) => void
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
  direction,
  onDirectionToggle,
  batches,
  selectedBatchId,
  onSelectBatch,
  trashCount,
  onTrashDrop,
}: Props) {
  const [collapsed, setCollapsed] = usePersistedState<Record<SectionId, boolean>>(
    "strata.rail.collapsed",
    {
      sort: false,
      content_colour: false,
      label: false,
      imports: false,
      system: false,
    },
  )
  const toggle = (id: SectionId) =>
    setCollapsed((prev) => ({ ...prev, [id]: !prev[id] }))

  return (
    <aside className="flex w-[220px] shrink-0 flex-col gap-4 overflow-y-auto border-r border-border bg-card/40 p-3 text-xs">
      <Section
        id="sort"
        title="Sort"
        collapsed={collapsed.sort}
        onToggle={() => toggle("sort")}
        summary={`${SORT_LABEL[sort]} ${direction === "asc" ? "↑" : "↓"}`}
      >
        <div className="flex gap-1">
          <select
            value={sort}
            onChange={(e) => onSortChange(e.target.value as SortKey)}
            className={cn(
              "h-7 flex-1 rounded-sm border border-border bg-background px-2 text-xs",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            )}
          >
            {SORT_KEYS.map((k) => (
              <option key={k} value={k}>
                {SORT_LABEL[k]}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={onDirectionToggle}
            aria-label={
              direction === "asc"
                ? "Sort ascending — click to descend"
                : "Sort descending — click to ascend"
            }
            title={direction === "asc" ? "Ascending" : "Descending"}
            className={cn(
              "inline-flex size-7 shrink-0 items-center justify-center rounded-sm border border-border bg-background text-muted-foreground",
              "hover:text-foreground hover:bg-muted/60",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            )}
          >
            {direction === "asc" ? (
              <ArrowUp weight="bold" className="size-3.5" />
            ) : (
              <ArrowDown weight="bold" className="size-3.5" />
            )}
          </button>
        </div>
      </Section>

      <Section
        id="content_colour"
        title="Content colour"
        collapsed={collapsed.content_colour}
        onToggle={() => toggle("content_colour")}
        summary={selectedBuckets.size > 0 ? selectedBuckets.size : undefined}
      >
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
      </Section>

      <Section
        id="label"
        title="Label"
        collapsed={collapsed.label}
        onToggle={() => toggle("label")}
        summary={selectedLabels.size > 0 ? selectedLabels.size : undefined}
      >
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
      </Section>

      <Section
        id="imports"
        title="Imports"
        collapsed={collapsed.imports}
        onToggle={() => toggle("imports")}
        summary={selectedBatchId ? 1 : undefined}
      >
        {batches.length === 0 ? (
          <p className="text-muted-foreground/70 px-1">(none yet)</p>
        ) : (
          <ul className="flex flex-col">
            {batches.map((b) => (
              <BatchRow
                key={b.id}
                batch={b}
                selected={selectedBatchId === b.id}
                onClick={() =>
                  onSelectBatch(selectedBatchId === b.id ? null : b.id)
                }
              />
            ))}
          </ul>
        )}
      </Section>

      <Section
        id="system"
        title="System"
        collapsed={collapsed.system}
        onToggle={() => toggle("system")}
        summary={trashCount > 0 ? trashCount : undefined}
      >
        <ul className="flex flex-col">
          <TrashRow count={trashCount} onDrop={onTrashDrop} />
        </ul>
      </Section>
    </aside>
  )
}

function TrashRow({
  count,
  onDrop,
}: {
  count: number
  onDrop: (ids: string[]) => void
}) {
  const [over, setOver] = useState(false)
  return (
    <li>
      <div
        aria-label={`Trash — ${count.toLocaleString()} items`}
        onDragOver={(e) => {
          if (!e.dataTransfer.types.includes(
            "application/x-strata-image",
          )) return
          e.preventDefault()
          e.dataTransfer.dropEffect = "move"
          if (!over) setOver(true)
        }}
        onDragEnter={(e) => {
          if (!e.dataTransfer.types.includes(
            "application/x-strata-image",
          )) return
          e.preventDefault()
          setOver(true)
        }}
        onDragLeave={(e) => {
          if (e.currentTarget.contains(e.relatedTarget as Node)) return
          setOver(false)
        }}
        onDrop={(e) => {
          const ids = readStrataImagePayload(e.dataTransfer)
          setOver(false)
          if (!ids || ids.length === 0) return
          e.preventDefault()
          onDrop(ids)
        }}
        className={cn(
          "flex w-full items-center gap-2 rounded-sm px-1 py-0.5 transition-colors",
          over
            ? "bg-destructive/15 text-destructive ring-1 ring-destructive/50"
            : "text-foreground",
        )}
      >
        <Trash
          weight="bold"
          className={cn(
            "size-3 shrink-0",
            over ? "text-destructive" : "text-muted-foreground",
          )}
        />
        <span className="flex-1 truncate">Trash</span>
        <span className="tabular-nums text-[10px] text-muted-foreground">
          {count.toLocaleString()}
        </span>
      </div>
    </li>
  )
}

function BatchRow({
  batch,
  selected,
  onClick,
}: {
  batch: BatchSummary
  selected: boolean
  onClick: () => void
}) {
  const date = new Date(batch.started_at)
  const folder = batch.source_folder.split("/").filter(Boolean).pop() ?? "—"
  const label = date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  })
  const time = date.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  })
  return (
    <li>
      <button
        type="button"
        aria-pressed={selected}
        onClick={onClick}
        title={batch.source_folder}
        className={cn(
          "flex w-full flex-col rounded-sm px-1 py-1 text-left transition-colors",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          selected
            ? "bg-muted text-foreground"
            : "text-foreground hover:bg-muted/60",
        )}
      >
        <span className="flex items-center justify-between gap-2">
          <span className="truncate">{label}</span>
          <span className="tabular-nums text-[10px] text-muted-foreground">
            {batch.image_count.toLocaleString()}
          </span>
        </span>
        <span className="flex items-center gap-1 text-[10px] text-muted-foreground/80">
          <span className="tabular-nums">{time}</span>
          <span>·</span>
          <span className="truncate">{folder}</span>
        </span>
      </button>
    </li>
  )
}

function Section({
  id,
  title,
  collapsed,
  onToggle,
  summary,
  children,
}: {
  id: SectionId
  title: string
  collapsed: boolean
  onToggle: () => void
  summary?: React.ReactNode
  children: React.ReactNode
}) {
  const bodyId = `rail-section-${id}-body`
  return (
    <section className="flex flex-col gap-1.5">
      <button
        type="button"
        aria-expanded={!collapsed}
        aria-controls={bodyId}
        onClick={onToggle}
        className={cn(
          "-mx-1 flex items-center gap-1 rounded-sm px-1 py-0.5 text-left",
          "hover:bg-muted/60",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        )}
      >
        <CaretRight
          weight="bold"
          className={cn(
            "size-3 shrink-0 text-muted-foreground transition-transform",
            !collapsed && "rotate-90",
          )}
        />
        <h2 className="font-heading flex-1 truncate text-[11px] font-semibold tracking-[0.08em] text-muted-foreground uppercase">
          {title}
        </h2>
        {collapsed && summary !== undefined && summary !== null && (
          <span className="text-[10px] text-muted-foreground/80">
            · {summary}
          </span>
        )}
      </button>
      {!collapsed && (
        <div id={bodyId} className="flex flex-col gap-1">
          {children}
        </div>
      )}
    </section>
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
