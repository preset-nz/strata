import { invoke } from "@tauri-apps/api/core"
import type { Vga16Bucket } from "@/lib/vga16"

export type ImportedRow = {
  id: string
  content_hash: string
  original_filename: string
  thumbnails_status: string
  dominant_bucket: Vga16Bucket | null
  dominant_l: number | null
  dominant_c: number | null
  dominant_h: number | null
}

export type ImageDetails = {
  id: string
  content_hash: string
  original_filename: string
  original_path: string
  byte_size: number
  mime: string
  imported_at: string
  ingest_batch_id: string
  thumbnails_status: string
  exif_created_at: string | null
  fs_mtime: string | null
  dominant_bucket: Vga16Bucket | null
  dominant_l: number | null
  dominant_c: number | null
  dominant_h: number | null
}

export function batchImported(batchId: string): Promise<ImportedRow[]> {
  return invoke<ImportedRow[]>("batch_imported", { batchId })
}

export function getImageDetails(id: string): Promise<ImageDetails | null> {
  return invoke<ImageDetails | null>("get_image_details", { id })
}
