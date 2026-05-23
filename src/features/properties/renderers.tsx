/* eslint-disable react-refresh/only-export-components --
 * Renderers + the registration entry point are colocated by design. */
import { registerFieldRenderer } from "@/properties"
import type { FieldRenderer } from "@/properties"
import {
  VGA16_HEX,
  VGA16_LABEL,
  type Vga16Bucket,
} from "@/lib/vga16"
import { COLOUR_SWATCH, type ColourLabel } from "@/components/image-card/colour-label"
import { Label } from "@/components/ui/label"

function FieldShell({
  label,
  children,
}: {
  label?: string
  children: React.ReactNode
}) {
  return (
    <div className="flex flex-col gap-1">
      {label && (
        <Label className="text-[11px] font-medium text-muted-foreground tracking-wide">
          {label}
        </Label>
      )}
      {children}
    </div>
  )
}

const Vga16BucketRenderer: FieldRenderer = ({ field, value }) => {
  const bucket = value as Vga16Bucket | null | undefined
  const hex = bucket ? VGA16_HEX[bucket] : null
  const label = bucket ? VGA16_LABEL[bucket] : null
  return (
    <FieldShell label={field.label ?? field.id}>
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
    </FieldShell>
  )
}

const ColourLabelRenderer: FieldRenderer = ({ field, value }) => {
  const colour = value as ColourLabel | null | undefined
  const swatch = colour ? COLOUR_SWATCH[colour] : null
  return (
    <FieldShell label={field.label ?? field.id}>
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
    </FieldShell>
  )
}

const StatusRenderer: FieldRenderer = ({ field, value }) => {
  const status = String(value ?? "")
  const tone = (() => {
    if (status === "ready") return "text-foreground"
    if (status === "missing" || status === "failed") return "text-destructive"
    return "text-muted-foreground"
  })()
  return (
    <FieldShell label={field.label ?? field.id}>
      <div className={`text-xs capitalize ${tone}`}>
        {status || <span className="text-muted-foreground">—</span>}
      </div>
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

const DateRenderer: FieldRenderer = ({ field, value }) => {
  const iso = value as string | null | undefined
  const formatted = iso ? DATE_FMT.format(new Date(iso)) : null
  return (
    <FieldShell label={field.label ?? field.id}>
      <span className="text-xs text-foreground tabular-nums">
        {formatted ?? <span className="text-muted-foreground">—</span>}
      </span>
    </FieldShell>
  )
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

const FileSizeRenderer: FieldRenderer = ({ field, value }) => {
  const bytes = value as number | null | undefined
  return (
    <FieldShell label={field.label ?? field.id}>
      <span className="text-xs text-foreground tabular-nums">
        {bytes != null ? formatBytes(bytes) : <span className="text-muted-foreground">—</span>}
      </span>
    </FieldShell>
  )
}

export function registerStrataRenderers(): void {
  registerFieldRenderer("vga16-bucket", Vga16BucketRenderer)
  registerFieldRenderer("colour-label", ColourLabelRenderer)
  registerFieldRenderer("status-pill", StatusRenderer)
  registerFieldRenderer("date", DateRenderer)
  registerFieldRenderer("file-size", FileSizeRenderer)
}
