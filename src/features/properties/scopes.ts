import { registerScope, type PropertySchema, type Scope, type ScopeContext } from "@preset.nz/facets"
import type { ImageDetails } from "@/features/contact-sheet/api"
import type { BatchSummary } from "@/features/library/api"

type ImageSelection = { kind: "image"; id: string }
type BatchSelection = { kind: "batch"; id: string }

const IMAGE_SCHEMA: PropertySchema = {
  version: 1,
  groups: [
    {
      id: "file",
      title: "File",
      rows: [
        { kind: "text", id: "displayTitle", label: "Title", path: "displayTitle" },
        { kind: "text", id: "filename", label: "Source filename", path: "filename" },
        { kind: "text", id: "originalPath", label: "Source path", path: "originalPath" },
        [
          { kind: "file-size", id: "byteSize", label: "Size", path: "byteSize" },
          { kind: "text", id: "mime", label: "Type", path: "mime" },
        ],
        { kind: "date", id: "importedAt", label: "Imported", path: "importedAt" },
        {
          kind: "status-pill",
          id: "status",
          label: "Thumbnail",
          path: "status",
        },
      ],
    },
    {
      id: "generation",
      title: "Generation",
      description: "How the image was made, read from the file.",
      rows: [
        { kind: "textarea", id: "prompt", label: "Prompt", path: "prompt", rows: 5 },
        { kind: "textarea", id: "negative", label: "Negative", path: "negative", rows: 2 },
        { kind: "text", id: "model", label: "Model", path: "model" },
        [
          { kind: "text", id: "seed", label: "Seed", path: "seed" },
          { kind: "text", id: "producer", label: "Made with", path: "producer" },
        ],
      ],
    },
    {
      id: "generationSettings",
      title: "Generation settings",
      collapsible: true,
      defaultCollapsed: true,
      rows: [
        [
          { kind: "text", id: "sampler", label: "Sampler", path: "sampler" },
          { kind: "text", id: "steps", label: "Steps", path: "steps" },
        ],
        [
          { kind: "text", id: "guidance", label: "Guidance", path: "guidance" },
          { kind: "text", id: "generatedSize", label: "Size", path: "generatedSize" },
        ],
        [
          { kind: "text", id: "strength", label: "Strength", path: "strength" },
          { kind: "text", id: "shift", label: "Shift", path: "shift" },
        ],
        { kind: "text", id: "seedMode", label: "Seed mode", path: "seedMode" },
      ],
    },
    {
      id: "dates",
      title: "Dates",
      rows: [
        {
          kind: "date",
          id: "exifCreatedAt",
          label: "Date taken (EXIF)",
          path: "exifCreatedAt",
        },
        {
          kind: "date",
          id: "iptcDateCreated",
          label: "Date created (IPTC)",
          path: "iptcDateCreated",
        },
      ],
    },
    {
      id: "camera",
      title: "Camera",
      rows: [
        { kind: "text", id: "cameraMake", label: "Make", path: "cameraMake" },
        { kind: "text", id: "cameraModel", label: "Model", path: "cameraModel" },
        { kind: "text", id: "lens", label: "Lens", path: "lens" },
      ],
    },
    {
      id: "exposure",
      title: "Exposure",
      rows: [
        [
          { kind: "text", id: "iso", label: "ISO", path: "iso" },
          { kind: "text", id: "aperture", label: "Aperture", path: "aperture" },
        ],
        [
          { kind: "text", id: "shutter", label: "Shutter", path: "shutter" },
          { kind: "text", id: "focalLength", label: "Focal", path: "focalLength" },
        ],
        [
          { kind: "text", id: "exposureBias", label: "Bias", path: "exposureBias" },
          { kind: "text", id: "exposureProgram", label: "Program", path: "exposureProgram" },
        ],
        [
          { kind: "text", id: "meteringMode", label: "Metering", path: "meteringMode" },
          { kind: "text", id: "flash", label: "Flash", path: "flash" },
        ],
      ],
    },
    {
      id: "image",
      title: "Image",
      rows: [
        {
          kind: "vector",
          id: "dimensions",
          label: "Dimensions (px)",
          path: "dimensions",
          integer: true,
          components: [{ label: "W" }, { label: "H" }],
        },
        [
          { kind: "text", id: "orientation", label: "Orientation", path: "orientation" },
          { kind: "text", id: "colorSpace", label: "Colour space", path: "colorSpace" },
        ],
      ],
    },
    {
      id: "location",
      title: "Location",
      rows: [
        {
          kind: "vector",
          id: "gpsLatLon",
          label: "Lat / Lon",
          path: "gpsLatLon",
          precision: 5,
          components: [
            { label: "Lat", suffix: "°" },
            { label: "Lon", suffix: "°" },
          ],
        },
        { kind: "text", id: "gpsAltitude", label: "Altitude", path: "gpsAltitude" },
        [
          { kind: "text", id: "city", label: "City", path: "city" },
          { kind: "text", id: "state", label: "State", path: "state" },
        ],
        { kind: "text", id: "country", label: "Country", path: "country" },
      ],
    },
    {
      id: "rights",
      title: "Rights",
      rows: [
        { kind: "text", id: "byline", label: "Creator", path: "byline" },
        { kind: "text", id: "copyright", label: "Copyright", path: "copyright" },
        { kind: "text", id: "title", label: "Title", path: "title" },
      ],
    },
    {
      id: "description",
      title: "Description",
      rows: [
        { kind: "textarea", id: "caption", label: "Caption", path: "caption", rows: 3 },
        { kind: "keyword-chips", id: "keywords", label: "Keywords", path: "keywords" },
      ],
    },
    {
      id: "colour",
      title: "Colour",
      description: "Dominant region of the image, in CIELCh.",
      rows: [
        {
          kind: "vga16-bucket",
          id: "dominantBucket",
          label: "Dominant bucket",
          path: "dominantBucket",
        },
        {
          kind: "vector",
          id: "dominantLCh",
          label: "CIELCh",
          path: "dominantLCh",
          precision: 2,
          components: [
            { label: "L*" },
            { label: "C*" },
            { label: "h°", suffix: "°" },
          ],
        },
      ],
    },
  ],
}

type ImageValues = {
  displayTitle: string | null
  filename: string | null
  prompt: string | null
  negative: string | null
  model: string | null
  seed: string | null
  producer: string | null
  sampler: string | null
  steps: string | null
  guidance: string | null
  generatedSize: string | null
  strength: string | null
  shift: string | null
  seedMode: string | null
  originalPath: string | null
  byteSize: number | null
  mime: string | null
  importedAt: string | null
  status: string | null
  exifCreatedAt: string | null
  iptcDateCreated: string | null
  cameraMake: string | null
  cameraModel: string | null
  lens: string | null
  iso: string | null
  aperture: string | null
  shutter: string | null
  focalLength: string | null
  exposureBias: string | null
  exposureProgram: string | null
  meteringMode: string | null
  flash: string | null
  dimensions: [number | null, number | null]
  orientation: string | null
  colorSpace: string | null
  gpsLatLon: [number | null, number | null]
  gpsAltitude: string | null
  city: string | null
  state: string | null
  country: string | null
  byline: string | null
  copyright: string | null
  title: string | null
  caption: string | null
  keywords: string[]
  dominantBucket: string | null
  dominantLCh: [number | null, number | null, number | null]
}

const PRODUCERS: Record<string, string> = { drawthings: "Draw Things" }

/** A scalar from the producer's settings, as display text. */
function setting(settings: Record<string, unknown> | null | undefined, key: string): string | null {
  const v = settings?.[key]
  if (v == null || typeof v === "object") return null
  if (typeof v === "number") return Number.isInteger(v) ? String(v) : String(Number(v.toFixed(3)))
  return String(v)
}

function formatShutter(sec: number | null | undefined): string | null {
  if (sec == null || !Number.isFinite(sec) || sec <= 0) return null
  if (sec >= 1) return `${sec.toFixed(sec >= 10 ? 0 : 1)} s`
  const denom = Math.round(1 / sec)
  return `1/${denom} s`
}

function formatAperture(f: number | null | undefined): string | null {
  if (f == null || !Number.isFinite(f)) return null
  return `f/${f.toFixed(f < 10 ? 1 : 0)}`
}

function formatFocalLength(
  mm: number | null | undefined,
  mm35: number | null | undefined,
): string | null {
  if (mm == null && mm35 == null) return null
  const main = mm != null ? `${mm.toFixed(0)} mm` : null
  const equiv = mm35 != null && mm35 !== mm ? ` (${mm35.toFixed(0)} mm eq.)` : ""
  return main ? main + equiv : mm35 != null ? `${mm35.toFixed(0)} mm eq.` : null
}

function formatBias(ev: number | null | undefined): string | null {
  if (ev == null || !Number.isFinite(ev)) return null
  const sign = ev > 0 ? "+" : ""
  return `${sign}${ev.toFixed(2)} EV`
}

function formatLens(make: string | null, model: string | null): string | null {
  if (model && make && !model.toLowerCase().includes(make.toLowerCase())) {
    return `${make} ${model}`
  }
  return model ?? make ?? null
}

function formatAltitude(m: number | null | undefined): string | null {
  if (m == null || !Number.isFinite(m)) return null
  return `${m.toFixed(0)} m`
}

const imageScope: Scope<ImageSelection, ImageValues> = {
  schema: IMAGE_SCHEMA,
  read: (_selection, ctx: ScopeContext) => {
    const d = ctx.details as ImageDetails | undefined
    const g = d?.generation ?? null
    return {
      displayTitle: d?.title ?? null,
      filename: d?.original_filename ?? null,
      prompt: g?.prompt ?? null,
      negative: g?.negative ?? null,
      model: g?.model ?? null,
      seed: g?.seed != null ? String(g.seed) : null,
      producer: g ? (PRODUCERS[g.producer] ?? g.producer) : null,
      sampler: setting(g?.settings, "sampler"),
      steps: setting(g?.settings, "steps"),
      guidance: setting(g?.settings, "scale"),
      generatedSize: setting(g?.settings, "size"),
      strength: setting(g?.settings, "strength"),
      shift: setting(g?.settings, "shift"),
      seedMode: setting(g?.settings, "seed_mode"),
      originalPath: d?.original_path ?? null,
      byteSize: d?.byte_size ?? null,
      mime: d?.mime ?? null,
      importedAt: d?.imported_at ?? null,
      status: d?.thumbnails_status ?? null,
      exifCreatedAt: d?.exif_created_at ?? null,
      iptcDateCreated: d?.iptc_date_created ?? null,
      cameraMake: d?.camera_make ?? null,
      cameraModel: d?.camera_model ?? null,
      lens: formatLens(d?.lens_make ?? null, d?.lens_model ?? null),
      iso: d?.iso != null ? String(d.iso) : null,
      aperture: formatAperture(d?.f_number),
      shutter: formatShutter(d?.exposure_time_sec),
      focalLength: formatFocalLength(d?.focal_length_mm, d?.focal_length_35mm),
      exposureBias: formatBias(d?.exposure_bias),
      exposureProgram: d?.exposure_program ?? null,
      meteringMode: d?.metering_mode ?? null,
      flash: d?.flash_fired == null ? null : d.flash_fired ? "Fired" : "No flash",
      dimensions: [d?.pixel_width ?? null, d?.pixel_height ?? null],
      orientation: d?.orientation != null ? String(d.orientation) : null,
      colorSpace: d?.color_space ?? null,
      gpsLatLon: [d?.gps_latitude ?? null, d?.gps_longitude ?? null],
      gpsAltitude: formatAltitude(d?.gps_altitude_m),
      city: d?.iptc_city ?? null,
      state: d?.iptc_state ?? null,
      country: d?.iptc_country ?? null,
      byline: d?.iptc_byline ?? null,
      copyright: d?.iptc_copyright ?? null,
      title: d?.iptc_title ?? null,
      caption: d?.iptc_caption ?? null,
      keywords: d?.keywords ?? [],
      dominantBucket: d?.dominant_bucket ?? null,
      dominantLCh: d ? [d.dominant_l, d.dominant_c, d.dominant_h] : [null, null, null],
    }
  },
}

const BATCH_SCHEMA: PropertySchema = {
  version: 1,
  groups: [
    {
      id: "identity",
      title: "Batch",
      rows: [
        { kind: "text", id: "id", label: "ID", path: "id" },
        {
          kind: "text",
          id: "sourceFolder",
          label: "Source folder",
          path: "sourceFolder",
        },
      ],
    },
    {
      id: "timing",
      title: "Timing",
      rows: [
        {
          kind: "text",
          id: "startedAt",
          label: "Started",
          path: "startedAt",
        },
        {
          kind: "text",
          id: "finishedAt",
          label: "Finished",
          path: "finishedAt",
        },
      ],
    },
    {
      id: "counts",
      title: "Counts",
      rows: [
        [
          {
            kind: "number",
            id: "imageCount",
            label: "Images",
            path: "imageCount",
          },
          {
            kind: "number",
            id: "importedCount",
            label: "Imported",
            path: "importedCount",
          },
        ],
        [
          {
            kind: "number",
            id: "skippedCount",
            label: "Skipped",
            path: "skippedCount",
          },
          {
            kind: "number",
            id: "failedCount",
            label: "Failed",
            path: "failedCount",
          },
        ],
      ],
    },
  ],
}

type BatchValues = {
  id: string | null
  sourceFolder: string | null
  startedAt: string | null
  finishedAt: string | null
  imageCount: number | null
  importedCount: number | null
  skippedCount: number | null
  failedCount: number | null
}

const batchScope: Scope<BatchSelection, BatchValues> = {
  schema: BATCH_SCHEMA,
  read: (_selection, ctx: ScopeContext) => {
    const b = ctx.batch as BatchSummary | undefined
    return {
      id: b?.id ?? null,
      sourceFolder: b?.source_folder ?? null,
      startedAt: b?.started_at ?? null,
      finishedAt: b?.finished_at ?? null,
      imageCount: b?.image_count ?? null,
      importedCount: b?.imported_count ?? null,
      skippedCount: b?.skipped_count ?? null,
      failedCount: b?.failed_count ?? null,
    }
  },
}

export function registerStrataScopes(): void {
  registerScope("image", imageScope)
  registerScope("batch", batchScope)
}
