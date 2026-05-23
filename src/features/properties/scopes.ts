import { registerScope, type PropertySchema, type Scope, type ScopeContext } from "@/properties"
import type { Selection } from "@/stores/selection"
import type { ImportedRow, ImageDetails } from "@/features/contact-sheet/api"
import type { BatchSummary } from "@/features/library/api"

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
  filename: string
  originalPath: string | null
  byteSize: number | null
  mime: string | null
  importedAt: string | null
  status: string
  exifCreatedAt: string | null
  dominantBucket: ImportedRow["dominant_bucket"]
  dominantLCh: [number | null, number | null, number | null]
}

const imageScope: Scope<
  Extract<Selection, { kind: "image" }>,
  ImageValues
> = {
  schema: IMAGE_SCHEMA,
  read: (selection, ctx: ScopeContext) => {
    const row = selection.row
    const details = ctx.details as ImageDetails | undefined
    return {
      filename: row.original_filename,
      originalPath: details?.original_path ?? null,
      byteSize: details?.byte_size ?? null,
      mime: details?.mime ?? null,
      importedAt: details?.imported_at ?? null,
      status: row.thumbnails_status,
      exifCreatedAt: details?.exif_created_at ?? null,
      dominantBucket: row.dominant_bucket,
      dominantLCh: [row.dominant_l, row.dominant_c, row.dominant_h],
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
  id: string
  sourceFolder: string
  startedAt: string
  finishedAt: string | null
  imageCount: number
  importedCount: number
  skippedCount: number
  failedCount: number
}

const batchScope: Scope<
  Extract<Selection, { kind: "batch" }>,
  BatchValues
> = {
  schema: BATCH_SCHEMA,
  read: (selection) => {
    const b: BatchSummary = selection.batch
    return {
      id: b.id,
      sourceFolder: b.source_folder,
      startedAt: b.started_at,
      finishedAt: b.finished_at,
      imageCount: b.image_count,
      importedCount: b.imported_count,
      skippedCount: b.skipped_count,
      failedCount: b.failed_count,
    }
  },
}

export function registerStrataScopes(): void {
  registerScope("image", imageScope)
  registerScope("batch", batchScope)
}
