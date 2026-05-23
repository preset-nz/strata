import { registerScope, type PropertySchema, type Scope } from "@/properties"
import type { Selection } from "@/stores/selection"
import type { ImportedRow } from "@/features/contact-sheet/api"
import type { BatchSummary } from "@/features/library/api"

const IMAGE_SCHEMA: PropertySchema = {
  version: 1,
  groups: [
    {
      id: "file",
      title: "File",
      rows: [
        { kind: "text", id: "filename", label: "Filename", path: "filename" },
        {
          kind: "text",
          id: "contentHash",
          label: "Content hash",
          path: "contentHash",
        },
        {
          kind: "status-pill",
          id: "status",
          label: "Thumbnail status",
          path: "status",
        },
      ],
    },
    {
      id: "colour",
      title: "Colour",
      rows: [
        {
          kind: "vga16-bucket",
          id: "dominantBucket",
          label: "Dominant bucket",
          path: "dominantBucket",
        },
        [
          {
            kind: "number",
            id: "dominantL",
            label: "L*",
            path: "dominantL",
          },
          {
            kind: "number",
            id: "dominantC",
            label: "C*",
            path: "dominantC",
          },
        ],
        {
          kind: "number",
          id: "dominantH",
          label: "h°",
          path: "dominantH",
        },
      ],
    },
  ],
}

type ImageValues = {
  filename: string
  contentHash: string
  status: string
  dominantBucket: ImportedRow["dominant_bucket"]
  dominantL: number | null
  dominantC: number | null
  dominantH: number | null
}

const imageScope: Scope<
  Extract<Selection, { kind: "image" }>,
  ImageValues
> = {
  schema: IMAGE_SCHEMA,
  read: (selection) => {
    const row = selection.row
    return {
      filename: row.original_filename,
      contentHash: row.content_hash,
      status: row.thumbnails_status,
      dominantBucket: row.dominant_bucket,
      dominantL: row.dominant_l,
      dominantC: row.dominant_c,
      dominantH: row.dominant_h,
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
