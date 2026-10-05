import { invoke } from "@tauri-apps/api/core"
import type { Vga16Bucket } from "@/lib/vga16"
import type { Orientation } from "@/lib/orientation"
import type { ImportedRow } from "../contact-sheet/api"
import type { SortKey } from "@/components/shell/sort-keys"

export type BucketCount = { bucket: Vga16Bucket; count: number }
export type OrientationCount = { orientation: Orientation; count: number }

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
  orientations?: Orientation[]
  /** "favourite" and colour labels; an image matching any of them shows. */
  labels?: string[]
  batchId?: string | null
  collectionId?: string | null
  projectKey?: string | null
  includeDeleted?: boolean
  onlyDeleted?: boolean
  /** Typed keywords. While set, results are ranked by relevance. */
  query?: string
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
    orientations: opts?.orientations,
    labels: opts?.labels,
    batchId: opts?.batchId ?? undefined,
    collectionId: opts?.collectionId ?? undefined,
    projectKey: opts?.projectKey ?? undefined,
    includeDeleted: opts?.includeDeleted,
    onlyDeleted: opts?.onlyDeleted,
    query: opts?.query || undefined,
  })
}

export function listBucketCounts(): Promise<BucketCount[]> {
  return invoke<BucketCount[]>("list_bucket_counts")
}

export function listOrientationCounts(): Promise<OrientationCount[]> {
  return invoke<OrientationCount[]>("list_orientation_counts")
}

export function listBatches(): Promise<BatchSummary[]> {
  return invoke<BatchSummary[]>("list_batches")
}

export function getBatch(id: string): Promise<BatchSummary | null> {
  return invoke<BatchSummary | null>("get_batch", { id })
}

export type CountQuery = {
  buckets?: Vga16Bucket[]
  orientations?: Orientation[]
  labels?: string[]
  batchId?: string | null
  collectionId?: string | null
  projectKey?: string | null
  includeDeleted?: boolean
  onlyDeleted?: boolean
  query?: string
}

export function deleteImages(ids: string[]): Promise<number> {
  return invoke<number>("delete_image", { ids })
}

export function restoreImages(ids: string[]): Promise<number> {
  return invoke<number>("restore_image", { ids })
}

export function purgeImages(ids: string[]): Promise<number> {
  return invoke<number>("purge_image", { ids })
}

export function libraryCount(opts?: CountQuery): Promise<number> {
  return invoke<number>("library_count", {
    buckets: opts?.buckets,
    orientations: opts?.orientations,
    labels: opts?.labels,
    batchId: opts?.batchId ?? undefined,
    collectionId: opts?.collectionId ?? undefined,
    projectKey: opts?.projectKey ?? undefined,
    includeDeleted: opts?.includeDeleted,
    onlyDeleted: opts?.onlyDeleted,
    query: opts?.query || undefined,
  })
}
