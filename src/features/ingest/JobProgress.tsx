import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

type FileState =
  | "queued"
  | "hashing"
  | "copying"
  | "verifying"
  | "indexing"
  | "thumbnailing"
  | "trashing"
  | "done"
  | "skipped_duplicate"
  | "failed";

type FileEvent = {
  batch_id: string;
  source_path: string;
  original_filename: string;
  state: FileState;
  content_hash?: string;
  image_id?: string;
  error?: string;
};

type BatchDoneEvent = {
  batch_id: string;
  imported: number;
  skipped: number;
  failed: number;
};

type Props = {
  batchId: string;
};

export function JobProgress({ batchId }: Props) {
  const [counts, setCounts] = useState({
    queued: 0,
    inflight: 0,
    done: 0,
    skipped: 0,
    failed: 0,
  });
  const [doneEvent, setDoneEvent] = useState<BatchDoneEvent | null>(null);

  useEffect(() => {
    let off1: UnlistenFn | undefined;
    let off2: UnlistenFn | undefined;
    (async () => {
      off1 = await listen<FileEvent>("ingest://file", (event) => {
        const ev = event.payload;
        if (ev.batch_id !== batchId) return;
        setCounts((c) => {
          const next = { ...c };
          switch (ev.state) {
            case "queued":
              next.queued += 1;
              break;
            case "hashing":
              next.queued -= 1;
              next.inflight += 1;
              break;
            case "done":
              next.inflight -= 1;
              next.done += 1;
              break;
            case "skipped_duplicate":
              next.skipped += 1;
              break;
            case "failed":
              next.inflight -= 1;
              next.failed += 1;
              break;
            default:
              break;
          }
          return next;
        });
      });
      off2 = await listen<BatchDoneEvent>("ingest://batch-done", (event) => {
        if (event.payload.batch_id !== batchId) return;
        setDoneEvent(event.payload);
      });
    })();
    return () => {
      off1?.();
      off2?.();
    };
  }, [batchId]);

  return (
    <section style={{ marginTop: 16 }}>
      <h2>Batch {batchId.slice(0, 8)}</h2>
      {doneEvent ? (
        <p>
          Imported {doneEvent.imported} - Skipped {doneEvent.skipped} - Failed{" "}
          {doneEvent.failed}
        </p>
      ) : (
        <p>
          Queued {counts.queued} - In-flight {counts.inflight} - Done {counts.done}{" "}
          - Skipped {counts.skipped} - Failed {counts.failed}
        </p>
      )}
    </section>
  );
}
