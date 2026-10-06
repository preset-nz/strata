/* eslint-disable react-refresh/only-export-components --
 * Renderers + the registration entry point are colocated by design. */
import {
  FieldShell,
  ReadOnlyText,
  registerFieldRenderer,
} from "@preset.nz/facets"
import type { FieldRenderer } from "@preset.nz/facets"
import { VGA16_HEX, VGA16_LABEL, type Vga16Bucket } from "@/lib/vga16"
import {
  COLOUR_SWATCH,
  type ColourLabel,
} from "@/components/image-card/colour-label"
import { PaletteExtract } from "./PaletteExtract"

const Vga16BucketRenderer: FieldRenderer = ({ field, value, view }) => {
  const bucket = value as Vga16Bucket | null | undefined
  const hex = bucket ? VGA16_HEX[bucket] : null
  const label = bucket ? VGA16_LABEL[bucket] : null
  return (
    <FieldShell label={field.label ?? field.id} view={view}>
      <ReadOnlyText>
        <div className="flex items-center gap-2">
          {hex ? (
            <span
              aria-hidden
              className="inline-block size-4 shrink-0 border border-border"
              style={{ backgroundColor: hex }}
            />
          ) : (
            <span className="inline-block size-4 shrink-0 border border-dashed border-border" />
          )}
          <span className="text-xs text-foreground">
            {label ?? <span className="text-muted-foreground">—</span>}
          </span>
        </div>
      </ReadOnlyText>
    </FieldShell>
  )
}

const ColourLabelRenderer: FieldRenderer = ({ field, value, view }) => {
  const colour = value as ColourLabel | null | undefined
  const swatch = colour ? COLOUR_SWATCH[colour] : null
  return (
    <FieldShell label={field.label ?? field.id} view={view}>
      <ReadOnlyText>
        <div className="flex items-center gap-2">
          {swatch ? (
            <span
              aria-hidden
              className="inline-block size-4 shrink-0 border border-border"
              style={{ backgroundColor: swatch }}
            />
          ) : (
            <span className="inline-block size-4 shrink-0 border border-dashed border-border" />
          )}
          <span className="text-xs text-foreground capitalize">
            {colour ?? <span className="text-muted-foreground">—</span>}
          </span>
        </div>
      </ReadOnlyText>
    </FieldShell>
  )
}

const StatusRenderer: FieldRenderer = ({ field, value, view }) => {
  const status = String(value ?? "")
  const tone = (() => {
    if (status === "ready") return "text-foreground"
    if (status === "missing" || status === "failed") return "text-destructive"
    return "text-muted-foreground"
  })()
  return (
    <FieldShell label={field.label ?? field.id} view={view}>
      <ReadOnlyText>
        <span className={`capitalize ${tone}`}>
          {status || <span className="text-muted-foreground">—</span>}
        </span>
      </ReadOnlyText>
    </FieldShell>
  )
}

const DATE_FMT = new Intl.DateTimeFormat(undefined, {
  year: "numeric",
  month: "short",
  day: "numeric",
  hour: "2-digit",
  minute: "2-digit",
})

const DateRenderer: FieldRenderer = ({ field, value, view }) => {
  const iso = value as string | null | undefined
  const formatted = iso ? DATE_FMT.format(new Date(iso)) : null
  return (
    <FieldShell label={field.label ?? field.id} view={view}>
      <ReadOnlyText>
        {formatted ?? <span className="text-muted-foreground">—</span>}
      </ReadOnlyText>
    </FieldShell>
  )
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

const FileSizeRenderer: FieldRenderer = ({ field, value, view }) => {
  const bytes = value as number | null | undefined
  return (
    <FieldShell label={field.label ?? field.id} view={view}>
      <ReadOnlyText>
        {bytes != null ? (
          formatBytes(bytes)
        ) : (
          <span className="text-muted-foreground">—</span>
        )}
      </ReadOnlyText>
    </FieldShell>
  )
}

const KeywordsRenderer: FieldRenderer = ({ field, value, view }) => {
  const list = Array.isArray(value) ? (value as string[]) : []
  return (
    <FieldShell label={field.label ?? field.id} view={view} top>
      {list.length === 0 ? (
        <ReadOnlyText>
          <span className="text-muted-foreground">—</span>
        </ReadOnlyText>
      ) : (
        <div className="flex flex-wrap gap-1 py-1.5">
          {list.map((kw) => (
            <span
              key={kw}
              className="inline-flex items-center border border-border bg-muted/40 px-1.5 py-0.5 text-[11px] text-foreground"
            >
              {kw}
            </span>
          ))}
        </div>
      )}
    </FieldShell>
  )
}

const PaletteExtractRenderer: FieldRenderer = ({ value }) => {
  const id = value as string | null | undefined
  if (!id) return null
  return <PaletteExtract key={id} imageId={id} />
}

export function registerStrataRenderers(): void {
  registerFieldRenderer("vga16-bucket", Vga16BucketRenderer)
  registerFieldRenderer("colour-label", ColourLabelRenderer)
  registerFieldRenderer("status-pill", StatusRenderer)
  registerFieldRenderer("date", DateRenderer)
  registerFieldRenderer("file-size", FileSizeRenderer)
  registerFieldRenderer("keyword-chips", KeywordsRenderer)
  registerFieldRenderer("palette-extract", PaletteExtractRenderer)
}
