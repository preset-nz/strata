import { invoke } from "@tauri-apps/api/core";

export type ImportedRow = {
  id: string;
  content_hash: string;
  original_filename: string;
  thumbnails_status: string;
};

export function batchImported(batchId: string): Promise<ImportedRow[]> {
  return invoke<ImportedRow[]>("batch_imported", { batchId });
}
