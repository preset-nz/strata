import { invoke } from "@tauri-apps/api/core";

export type PrescanSummary = {
  root: string;
  total: number;
  by_extension: Record<string, number>;
};

export type StartBatchResult = {
  batch_id: string;
  total: number;
};

export function prescan(path: string): Promise<PrescanSummary> {
  return invoke<PrescanSummary>("ingest_prescan", { path });
}

export function startIngest(path: string): Promise<StartBatchResult> {
  return invoke<StartBatchResult>("ingest_start", { path });
}
