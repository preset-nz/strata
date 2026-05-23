import { invoke } from "@tauri-apps/api/core";
import type { ImportedRow } from "../contact-sheet/api";

export function listImages(offset: number, limit: number): Promise<ImportedRow[]> {
  return invoke<ImportedRow[]>("list_images", { offset, limit });
}
