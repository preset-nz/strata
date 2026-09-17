import { useState } from "react"
import {
  ArrowDown,
  ArrowUp,
  CaretRight,
  Heart,
  Images,
  Trash,
} from "@phosphor-icons/react"
import {
  ORIENTATIONS,
  ORIENTATION_LABEL,
  type Orientation,
} from "@/lib/orientation"
import { cn } from "@/lib/utils"
import { readStrataImagePayload } from "@/components/image-card/use-draggable-card"
import { usePersistedState } from "@preset.nz/preferences"
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
import {
  SORT_LABEL,
  type SortDirection,
  type SortKey,
} from "./sort-keys"

type SectionId =
  | "sort"
  | "orientation"
  | "content_colour"
  | "label"
  | "imports"
  | "system"

type LabelSelector = "favourite" | ColourLabel

type Props = {
  orientationCounts: Record<Orientation, number>
  selectedOrientations: Set<Orientation>
  onToggleOrientation: (orientation: Orientation) => void
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
  sortOptions: SortKey[]
  batches: BatchSummary[]
  selectedBatchId: string | null
  onSelectBatch: (id: string | null) => void
  trashCount: number
  onTrashDrop: (ids: string[]) => void
  trashActive: boolean
  onSelectTrash: () => void
  libraryTotal: number
  libraryActive: boolean
  onSelectLibrary: () => void
  /** When true, filter sections (Content colour / Label / Imports) are dimmed
   * and read-only — they don't apply to the active panel (e.g. Trash). The
   * underlying selection state is preserved so returning to Library restores
   * any pending filters. */
  filtersDisabled: boolean
}

export function LeftRail({
  orientationCounts,
  selectedOrientations,
  onToggleOrientation,
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
  sortOptions,
  batches,
  selectedBatchId,
  onSelectBatch,
  trashCount,
  onTrashDrop,
  trashActive,
  onSelectTrash,
  libraryTotal,
  libraryActive,
  onSelectLibrary,
  filtersDisabled,
}: Props) {
  const [collapsed, setCollapsed] = usePersistedState<Record<SectionId, boolean>>(
    "strata.rail.collapsed",
    {
      sort: false,
      orientation: false,
      content_colour: false,
      label: false,
      imports: false,
      system: false,
    },
  )
  const toggle = (id: SectionId) =>
    setCollapsed((prev) => ({ ...prev, [id]: !prev[id] }))

  return (
    <aside className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-3 text-xs">
      <NavRow
        icon={
          <Images
            weight="bold"
            className={cn(
              "size-3 shrink-0",
              libraryActive ? "text-foreground" : "text-muted-foreground",
            )}
          />
        }
        label="Library"
        count={libraryTotal}
        active={libraryActive}
        onClick={onSelectLibrary}
      />
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
            disabled={sortOptions.length <= 1}
            className={cn(
              "h-7 flex-1 rounded-sm border border-border bg-background px-2 text-xs",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
              "disabled:cursor-default disabled:opacity-80",
            )}
          >
            {sortOptions.map((k) => (
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
        id="orientation"
        title="Orientation"
        collapsed={collapsed.orientation}
        onToggle={() => toggle("orientation")}
        summary={
          selectedOrientations.size > 0 ? selectedOrientations.size : undefined
        }
      >
        <ul className="flex flex-col">
          {ORIENTATIONS.map((o) => (
            <LabelRow
              key={o}
              id={o}
              label={ORIENTATION_LABEL[o]}
              count={orientationCounts[o] ?? 0}
              selected={selectedOrientations.has(o)}
              disabled={filtersDisabled}
              onClick={() => onToggleOrientation(o)}
              swatch={<OrientationGlyph orientation={o} />}
            />
          ))}
        </ul>
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
                  disabled={empty || filtersDisabled}
                  onClick={() => onToggleBucket(b)}
                  className={cn(
                    "group flex w-full items-center gap-2 rounded-sm px-1 py-0.5 text-left transition-colors",
                    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                    selected
                      ? "bg-muted text-foreground"
                      : empty
                        ? "text-muted-foreground/40"
                        : "text-foreground hover:bg-muted/60",
                    filtersDisabled && "opacity-60 cursor-default hover:bg-transparent",
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
            disabled={filtersDisabled}
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
              disabled={filtersDisabled}
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
                disabled={filtersDisabled}
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
          <TrashRow
            count={trashCount}
            active={trashActive}
            onClick={onSelectTrash}
            onDrop={onTrashDrop}
          />
        </ul>
      </Section>
    </aside>
  )
}

function TrashRow({
  count,
  active,
  onClick,
  onDrop,
}: {
  count: number
  active: boolean
  onClick: () => void
  onDrop: (ids: string[]) => void
}) {
  const [over, setOver] = useState(false)
  return (
    <li>
      <button
        type="button"
        aria-pressed={active}
        aria-label={`Trash — ${count.toLocaleString()} items`}
        onClick={onClick}
        // WebKit (Tauri's macOS engine) hides dataTransfer.types during
        // dragenter/dragover for security — we can't filter on MIME until the
        // drop fires. Always preventDefault to enable the drop and paint the
        // hover state; validate the payload at drop time and ignore non-ours.
        onDragOver={(e) => {
          e.preventDefault()
          e.dataTransfer.dropEffect = "move"
          if (!over) setOver(true)
        }}
        onDragEnter={(e) => {
          e.preventDefault()
          setOver(true)
        }}
        onDragLeave={(e) => {
          if (e.currentTarget.contains(e.relatedTarget as Node)) return
          setOver(false)
        }}
        onDrop={(e) => {
          e.preventDefault()
          setOver(false)
          const ids = readStrataImagePayload(e.dataTransfer)
          if (!ids || ids.length === 0) return
          onDrop(ids)
        }}
        className={cn(
          "flex w-full items-center gap-2 rounded-sm px-1 py-0.5 text-left",
          "transition-[background-color,color,box-shadow,transform] duration-100",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          over
            ? "scale-[1.03] bg-destructive/25 text-destructive ring-2 ring-destructive shadow-sm"
            : active
              ? "bg-muted text-foreground"
              : "text-foreground hover:bg-muted/60",
        )}
      >
        <Trash
          weight={over ? "fill" : "bold"}
          className={cn(
            "size-3 shrink-0",
            over ? "text-destructive" : "text-muted-foreground",
          )}
        />
        <span className="flex-1 truncate">
          {over ? "Drop to delete" : "Trash"}
        </span>
        <span
          className={cn(
            "tabular-nums text-[10px]",
            over ? "text-destructive" : "text-muted-foreground",
          )}
        >
          {count.toLocaleString()}
        </span>
      </button>
    </li>
  )
}

function BatchRow({
  batch,
  selected,
  disabled,
  onClick,
}: {
  batch: BatchSummary
  selected: boolean
  disabled?: boolean
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
        disabled={disabled}
        onClick={onClick}
        title={batch.source_folder}
        className={cn(
          "flex w-full flex-col rounded-sm px-1 py-1 text-left transition-colors",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          selected
            ? "bg-muted text-foreground"
            : "text-foreground hover:bg-muted/60",
          disabled && "opacity-60 cursor-default hover:bg-transparent",
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

// A 12px frame whose aspect matches the bucket: wide, equal, tall.
function OrientationGlyph({ orientation }: { orientation: Orientation }) {
  const size =
    orientation === "landscape"
      ? "h-2 w-3"
      : orientation === "portrait"
        ? "h-3 w-2"
        : "size-2.5"
  return (
    <span className="inline-flex size-3 shrink-0 items-center justify-center">
      <span
        className={cn(
          "rounded-[1px] border border-foreground/60",
          size,
        )}
      />
    </span>
  )
}

function LabelRow({
  label,
  count,
  selected,
  disabled = false,
  onClick,
  swatch,
}: {
  id: LabelSelector | Orientation
  label: string
  count: number
  selected: boolean
  disabled?: boolean
  onClick: () => void
  swatch: React.ReactNode
}) {
  const empty = count === 0 && !selected
  return (
    <li>
      <button
        type="button"
        aria-pressed={selected}
        disabled={empty || disabled}
        onClick={onClick}
        className={cn(
          "flex w-full items-center gap-2 rounded-sm px-1 py-0.5 text-left transition-colors",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          selected
            ? "bg-muted text-foreground"
            : empty
              ? "text-muted-foreground/40"
              : "text-foreground hover:bg-muted/60",
          disabled && "opacity-60 cursor-default hover:bg-transparent",
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

function NavRow({
  icon,
  label,
  count,
  active,
  onClick,
}: {
  icon: React.ReactNode
  label: string
  count: number
  active: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={cn(
        "-mx-1 flex items-center gap-2 rounded-sm px-2 py-1 text-left transition-colors",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        active
          ? "bg-muted text-foreground"
          : "text-foreground hover:bg-muted/60",
      )}
    >
      {icon}
      <span className="flex-1 truncate font-medium">{label}</span>
      <span className="tabular-nums text-[10px] text-muted-foreground">
        {count.toLocaleString()}
      </span>
    </button>
  )
}
