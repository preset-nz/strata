import { registerScope, type PropertySchema, type Scope, type ScopeContext } from "@/properties"
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
        { kind: "text", id: "filename", label: "Filename", path: "filename" },
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
      id: "dates",
      title: "Dates",
      rows: [
        {
          kind: "date",
          id: "exifCreatedAt",
          label: "Date taken",
          path: "exifCreatedAt",
        },
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
  filename: string | null
  originalPath: string | null
  byteSize: number | null
  mime: string | null
  importedAt: string | null
  status: string | null
  exifCreatedAt: string | null
  dominantBucket: string | null
  dominantLCh: [number | null, number | null, number | null]
}

const imageScope: Scope<ImageSelection, ImageValues> = {
  schema: IMAGE_SCHEMA,
  read: (_selection, ctx: ScopeContext) => {
    const d = ctx.details as ImageDetails | undefined
    return {
      filename: d?.original_filename ?? null,
      originalPath: d?.original_path ?? null,
      byteSize: d?.byte_size ?? null,
      mime: d?.mime ?? null,
      importedAt: d?.imported_at ?? null,
      status: d?.thumbnails_status ?? null,
      exifCreatedAt: d?.exif_created_at ?? null,
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
