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

export function batchImported(batchId: string): Promise<ImportedRow[]> {
  return invoke<ImportedRow[]>("batch_imported", { batchId })
}
