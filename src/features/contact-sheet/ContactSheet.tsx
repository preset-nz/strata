import { useEffect, useMemo, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Thumbnail } from "./Thumbnail";
import { ThumbGrid } from "./ThumbGrid";
import { batchImported, type ImportedRow } from "./api";

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

type ImportedCell = {
  key: string;
  id?: string;
  hash: string;
  filename: string;
  status: string;
};

type SkippedCell = {
  key: string;
  hash: string;
  filename: string;
  existingId: string;
};

type FailedCell = {
  key: string;
  filename: string;
  sourcePath: string;
  error: string;
};

type Props = {
  batchId: string;
};

export function ContactSheet({ batchId }: Props) {
  const [imported, setImported] = useState<Map<string, ImportedCell>>(new Map());
  const [skipped, setSkipped] = useState<SkippedCell[]>([]);
  const [failed, setFailed] = useState<FailedCell[]>([]);
  const [done, setDone] = useState<BatchDoneEvent | null>(null);

  useEffect(() => {
    let cancelled = false;
    batchImported(batchId)
      .then((rows) => {
        if (cancelled) return;
        setImported((prev) => mergeRows(prev, rows));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [batchId]);

  useEffect(() => {
    let off1: UnlistenFn | undefined;
    let off2: UnlistenFn | undefined;
    (async () => {
      off1 = await listen<FileEvent>("ingest://file", (event) => {
        const ev = event.payload;
        if (ev.batch_id !== batchId) return;
        if (ev.state === "done" && ev.image_id && ev.content_hash) {
          setImported((prev) => {
            const next = new Map(prev);
            const existing = next.get(ev.image_id!);
            next.set(ev.image_id!, {
              key: ev.image_id!,
              id: ev.image_id,
              hash: ev.content_hash!,
              filename: ev.original_filename,
              status: existing?.status === "ready" ? "ready" : "thumbnailing",
            });
            return next;
          });
        } else if (ev.state === "skipped_duplicate" && ev.content_hash) {
          setSkipped((prev) => [
            ...prev,
            {
              key: ev.source_path,
              hash: ev.content_hash!,
              filename: ev.original_filename,
              existingId: ev.image_id ?? "",
            },
          ]);
        } else if (ev.state === "failed") {
          setFailed((prev) => [
            ...prev,
            {
              key: ev.source_path,
              filename: ev.original_filename,
              sourcePath: ev.source_path,
              error: ev.error ?? "unknown",
            },
          ]);
        }
      });
      off2 = await listen<BatchDoneEvent>("ingest://batch-done", async (event) => {
        if (event.payload.batch_id !== batchId) return;
        setDone(event.payload);
        try {
          const rows = await batchImported(batchId);
          setImported((prev) => mergeRows(prev, rows));
        } catch {}
      });
    })();
    return () => {
      off1?.();
      off2?.();
    };
  }, [batchId]);

  const importedList = useMemo(
    () => Array.from(imported.values()),
    [imported],
  );

  return (
    <section style={{ marginTop: 16 }}>
      <h2 style={{ margin: "8px 0" }}>
        Last import
        {done && (
          <span style={{ fontWeight: "normal", marginLeft: 8, fontSize: 14, opacity: 0.8 }}>
            Imported {done.imported} - Skipped {done.skipped} - Failed {done.failed}
          </span>
        )}
      </h2>

      <Section label={`Imported (${importedList.length})`}>
        <ThumbGrid
          items={importedList}
          renderCell={(it) => (
            <Thumbnail hash={it.hash} filename={it.filename} status={it.status} />
          )}
        />
      </Section>

      <Section label={`Skipped - already in catalog (${skipped.length})`}>
        <ThumbGrid
          items={skipped}
          renderCell={(it) => (
            <Thumbnail hash={it.hash} filename={it.filename} status="ready" />
          )}
        />
      </Section>

      <Section label={`Failed (${failed.length})`}>
        <ul style={{ margin: 0, paddingLeft: 20 }}>
          {failed.map((f) => (
            <li key={f.key} style={{ fontSize: 12 }}>
              <code>{f.filename}</code> - {f.error}{" "}
              <span style={{ opacity: 0.6 }}>({f.sourcePath})</span>
            </li>
          ))}
        </ul>
      </Section>
    </section>
  );
}

function mergeRows(prev: Map<string, ImportedCell>, rows: ImportedRow[]) {
  const next = new Map(prev);
  for (const row of rows) {
    next.set(row.id, {
      key: row.id,
      id: row.id,
      hash: row.content_hash,
      filename: row.original_filename,
      status: row.thumbnails_status,
    });
  }
  return next;
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 24 }}>
      <h3 style={{ margin: "8px 0", fontSize: 14, opacity: 0.85 }}>{label}</h3>
      {children}
    </div>
  );
}

