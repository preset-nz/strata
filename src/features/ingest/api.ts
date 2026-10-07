import { invoke } from "@tauri-apps/api/core"

/** One folder in the pre-scan, relative to the root ("" is the root). */
export type FolderRow = {
  path: string
  name: string
  depth: number
  /** Importable files directly in this folder. */
  own: number
  /** Importable files here and below. */
  total: number
}

export type PrescanSummary = {
  root: string
  total: number
  by_extension: Record<string, number>
  folders: FolderRow[]
  keep_source_files: boolean
}

export type StartBatchResult = {
  batch_id: string
  total: number
}

export function prescan(path: string): Promise<PrescanSummary> {
  return invoke<PrescanSummary>("ingest_prescan", { path })
}

/** Imports the files directly in `folders`, from the last pre-scan of `path`. */
export function startIngest(
  path: string,
  folders: string[]
): Promise<StartBatchResult> {
  return invoke<StartBatchResult>("ingest_start", { path, folders })
}
