import { invoke } from "@tauri-apps/api/core"
import type { Vga16Bucket } from "@/lib/vga16"

export type ImportedRow = {
  id: string
  content_hash: string
  original_filename: string
  /** The prompt head for a generated image, else the filename. */
  title: string
  favourite: boolean
  label: string | null
  thumbnails_status: string
  dominant_bucket: Vga16Bucket | null
  dominant_l: number | null
  dominant_c: number | null
  dominant_h: number | null
  deleted_at: string | null
}

/** How a generated image was made, from its provenance record. */
export type GenerationDetails = {
  producer: string
  prompt: string
  negative: string | null
  model: string | null
  seed: number | null
  /** The producer's own settings, as it wrote them. */
  settings: Record<string, unknown> | null
}

export type ImageDetails = {
  id: string
  content_hash: string
  original_filename: string
  title: string
  generation: GenerationDetails | null
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

  camera_make: string | null
  camera_model: string | null
  lens_make: string | null
  lens_model: string | null
  focal_length_mm: number | null
  focal_length_35mm: number | null

  iso: number | null
  f_number: number | null
  exposure_time_sec: number | null
  exposure_bias: number | null
  exposure_program: string | null
  metering_mode: string | null
  flash_fired: boolean | null

  pixel_width: number | null
  pixel_height: number | null
  orientation: number | null
  color_space: string | null

  gps_latitude: number | null
  gps_longitude: number | null
  gps_altitude_m: number | null

  iptc_title: string | null
  iptc_caption: string | null
  iptc_byline: string | null
  iptc_copyright: string | null
  iptc_city: string | null
  iptc_state: string | null
  iptc_country: string | null
  iptc_date_created: string | null

  software: string | null
  keywords: string[]
}

export function batchImported(batchId: string): Promise<ImportedRow[]> {
  return invoke<ImportedRow[]>("batch_imported", { batchId })
}

export function getImageDetails(id: string): Promise<ImageDetails | null> {
  return invoke<ImageDetails | null>("get_image_details", { id })
}
