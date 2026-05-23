import { invoke } from "@tauri-apps/api/core"
import type { Vga16Bucket } from "@/lib/vga16"
import type { ImportedRow } from "../contact-sheet/api"
import type { SortKey } from "@/components/shell/LeftRail"

export type BucketCount = { bucket: Vga16Bucket; count: number }

export function listImages(
  offset: number,
  limit: number,
  opts?: { sort?: SortKey; buckets?: Vga16Bucket[] },
): Promise<ImportedRow[]> {
  return invoke<ImportedRow[]>("list_images", {
    offset,
    limit,
    sort: opts?.sort,
    buckets: opts?.buckets,
  })
}

export function listBucketCounts(): Promise<BucketCount[]> {
  return invoke<BucketCount[]>("list_bucket_counts")
}

export function libraryCount(buckets?: Vga16Bucket[]): Promise<number> {
  return invoke<number>("library_count", { buckets })
}
