import { invoke } from "@tauri-apps/api/core"
import type { Vga16Bucket } from "@/lib/vga16"
import type { ImportedRow } from "../contact-sheet/api"
import type { SortKey } from "@/components/shell/LeftRail"

export type BucketCount = { bucket: Vga16Bucket; count: number }

export type BatchSummary = {
  id: string
  source_folder: string
  started_at: string
  finished_at: string | null
  imported_count: number
  skipped_count: number
  failed_count: number
  image_count: number
}

export type SortDirection = "asc" | "desc"

export type ImagesQuery = {
  sort?: SortKey
  direction?: SortDirection
  buckets?: Vga16Bucket[]
  batchId?: string | null
}

export function listImages(
  offset: number,
  limit: number,
  opts?: ImagesQuery,
): Promise<ImportedRow[]> {
  return invoke<ImportedRow[]>("list_images", {
    offset,
    limit,
    sort: opts?.sort,
    direction: opts?.direction,
    buckets: opts?.buckets,
    batchId: opts?.batchId ?? undefined,
  })
}

export function listBucketCounts(): Promise<BucketCount[]> {
  return invoke<BucketCount[]>("list_bucket_counts")
}

export function listBatches(): Promise<BatchSummary[]> {
  return invoke<BatchSummary[]>("list_batches")
}

export function getBatch(id: string): Promise<BatchSummary | null> {
  return invoke<BatchSummary | null>("get_batch", { id })
}

export function libraryCount(
  buckets?: Vga16Bucket[],
  batchId?: string | null,
): Promise<number> {
  return invoke<number>("library_count", {
    buckets,
    batchId: batchId ?? undefined,
  })
}
