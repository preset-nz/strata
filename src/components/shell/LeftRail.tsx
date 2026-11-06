import { Heart, Images, Trash } from "@phosphor-icons/react"
import { usePersistedState } from "@preset.nz/preferences"
import { Rail, RailRow, RailSection, RailSort } from "@preset.nz/ux-kit"
import { useState } from "react"
import {
  COLOUR_LABELS,
  COLOUR_SWATCH,
  type ColourLabel,
} from "@/components/image-card/colour-label"
import { readStrataImagePayload } from "@/components/image-card/use-draggable-card"
import type { BatchSummary } from "@/features/library/api"
import {
  ORIENTATION_LABEL,
  ORIENTATIONS,
  type Orientation,
} from "@/lib/orientation"
import { cn } from "@/lib/utils"
import {
  VGA16_BUCKETS,
  VGA16_HEX,
  VGA16_LABEL,
  type Vga16Bucket,
} from "@/lib/vga16"
import { SORT_LABEL, type SortDirection, type SortKey } from "./sort-keys"

const SORT_OPTIONS = (keys: SortKey[]) =>
  keys.map((key) => ({ key, label: SORT_LABEL[key] }))

type SectionId =
  | "searches"
  | "collections"
  | "projects"
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
  /** The Searches section's body, and how many saved searches there are. */
  savedSearches: React.ReactNode
  savedSearchCount: number
  /** Holds Searches open while a name is being typed into it. */
  savedSearchesOpen?: boolean
  /** The Projects section's body and its count of live (unarchived) projects. */
  projects: React.ReactNode
  projectCount: number
  /** The Collections section's body and count; held open while naming. */
  collections: React.ReactNode
  collectionCount: number
  collectionsOpen?: boolean
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
  savedSearches,
  savedSearchCount,
  savedSearchesOpen,
  collections,
  collectionCount,
  collectionsOpen,
  projects,
  projectCount,
}: Props) {
  const [collapsed, setCollapsed] = usePersistedState<
    Record<SectionId, boolean>
  >("strata.rail.collapsed", {
    searches: false,
    collections: false,
    projects: false,
    sort: false,
    orientation: false,
    content_colour: false,
    label: false,
    imports: false,
    system: false,
  })
  const toggle = (id: SectionId) =>
    setCollapsed((prev) => ({ ...prev, [id]: !prev[id] }))

  return (
    <Rail>
      <RailRow
        variant="nav"
        leading={
          <Images
            weight="bold"
            className={cn(
              "size-3 shrink-0",
              libraryActive ? "text-foreground" : "text-muted-foreground"
            )}
          />
        }
        label="Library"
        count={libraryTotal}
        selected={libraryActive}
        onClick={onSelectLibrary}
      />
      <RailSection
        id="searches"
        title="Searches"
        collapsed={collapsed.searches && !savedSearchesOpen}
        onToggle={() => toggle("searches")}
        summary={savedSearchCount > 0 ? savedSearchCount : undefined}
      >
        {savedSearches}
      </RailSection>
      <RailSection
        id="projects"
        title="Projects"
        collapsed={collapsed.projects}
        onToggle={() => toggle("projects")}
        summary={projectCount > 0 ? projectCount : undefined}
      >
        {projects}
      </RailSection>
      <RailSection
        id="collections"
        title="Collections"
        collapsed={collapsed.collections && !collectionsOpen}
        onToggle={() => toggle("collections")}
        summary={collectionCount > 0 ? collectionCount : undefined}
      >
        {collections}
      </RailSection>
      <RailSection
        id="sort"
        title="Sort"
        collapsed={collapsed.sort}
        onToggle={() => toggle("sort")}
        summary={`${SORT_LABEL[sort]} ${direction === "asc" ? "↑" : "↓"}`}
      >
        <RailSort
          value={sort}
          onValueChange={onSortChange}
          options={SORT_OPTIONS(sortOptions)}
          direction={direction}
          onDirectionToggle={onDirectionToggle}
        />
      </RailSection>

      <RailSection
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
            <li key={o}>
              <RailRow
                label={ORIENTATION_LABEL[o]}
                count={orientationCounts[o] ?? 0}
                selected={selectedOrientations.has(o)}
                readOnly={filtersDisabled}
                onClick={() => onToggleOrientation(o)}
                leading={<OrientationGlyph orientation={o} />}
              />
            </li>
          ))}
        </ul>
      </RailSection>

      <RailSection
        id="content_colour"
        title="Content colour"
        collapsed={collapsed.content_colour}
        onToggle={() => toggle("content_colour")}
        summary={selectedBuckets.size > 0 ? selectedBuckets.size : undefined}
      >
        <ul className="flex flex-col">
          {VGA16_BUCKETS.map((b) => {
            const selected = selectedBuckets.has(b)
            return (
              <li key={b}>
                <RailRow
                  label={VGA16_LABEL[b]}
                  count={bucketCounts[b] ?? 0}
                  selected={selected}
                  readOnly={filtersDisabled}
                  onClick={() => onToggleBucket(b)}
                  leading={
                    <>
                      <span
                        className={cn(
                          "inline-flex size-3 shrink-0 items-center justify-center rounded-[2px] border",
                          selected
                            ? "border-foreground/60 bg-foreground/10"
                            : "border-border"
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
                    </>
                  }
                />
              </li>
            )
          })}
        </ul>
      </RailSection>

      <RailSection
        id="label"
        title="Label"
        collapsed={collapsed.label}
        onToggle={() => toggle("label")}
        summary={selectedLabels.size > 0 ? selectedLabels.size : undefined}
      >
        <ul className="flex flex-col">
          <li>
            <RailRow
              label="Favourited"
              count={labelCounts.favourite ?? 0}
              selected={selectedLabels.has("favourite")}
              readOnly={filtersDisabled}
              onClick={() => onToggleLabel("favourite")}
              leading={
                <Heart weight="fill" className="size-3 shrink-0 text-red-500" />
              }
            />
          </li>
          {COLOUR_LABELS.map((c) => (
            <li key={c}>
              <RailRow
                label={c.charAt(0).toUpperCase() + c.slice(1)}
                count={labelCounts[c] ?? 0}
                selected={selectedLabels.has(c)}
                readOnly={filtersDisabled}
                onClick={() => onToggleLabel(c)}
                leading={
                  <span
                    className={cn(
                      "inline-block size-3 shrink-0 rounded-full",
                      COLOUR_SWATCH[c]
                    )}
                  />
                }
              />
            </li>
          ))}
        </ul>
      </RailSection>

      <RailSection
        id="imports"
        title="Imports"
        collapsed={collapsed.imports}
        onToggle={() => toggle("imports")}
        summary={selectedBatchId ? 1 : undefined}
      >
        {batches.length === 0 ? (
          <p className="px-1 text-muted-foreground/70">(none yet)</p>
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
      </RailSection>

      <RailSection
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
      </RailSection>
    </Rail>
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
            ? "scale-[1.03] bg-destructive/25 text-destructive shadow-sm ring-2 ring-destructive"
            : active
              ? "bg-muted text-foreground"
              : "text-foreground hover:bg-muted/60"
        )}
      >
        <Trash
          weight={over ? "fill" : "bold"}
          className={cn(
            "size-3 shrink-0",
            over ? "text-destructive" : "text-muted-foreground"
          )}
        />
        <span className="flex-1 truncate">
          {over ? "Drop to delete" : "Trash"}
        </span>
        <span
          className={cn(
            "text-[10px] tabular-nums",
            over ? "text-destructive" : "text-muted-foreground"
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
          disabled && "cursor-default opacity-60 hover:bg-transparent"
        )}
      >
        <span className="flex items-center justify-between gap-2">
          <span className="truncate">{label}</span>
          <span className="text-[10px] text-muted-foreground tabular-nums">
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
      <span className={cn("rounded-[1px] border border-foreground/60", size)} />
    </span>
  )
}
