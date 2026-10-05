import { useEffect, useMemo, useState } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { ImageCard } from "@/components/image-card"
import { MarkedImageCard } from "@/features/curation/MarkedImageCard"
import { loadMarks, putRows } from "@/features/curation/marks"
import { ThumbGrid } from "./ThumbGrid"
import { batchImported, type ImportedRow } from "./api"

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
  | "failed"

type FileEvent = {
  batch_id: string
  source_path: string
  original_filename: string
  state: FileState
  content_hash?: string
  image_id?: string
  error?: string
}

type BatchDoneEvent = {
  batch_id: string
  imported: number
  skipped: number
  failed: number
}

type ImportedCell = {
  key: string
  id?: string
  hash: string
  filename: string
  status: string
}

type SkippedCell = {
  key: string
  hash: string
  filename: string
  existingId: string
}

type FailedCell = {
  key: string
  filename: string
  sourcePath: string
  error: string
}

type Props = {
  batchId: string
  cellSize?: number
}

export function ContactSheet({ batchId, cellSize }: Props) {
  const [imported, setImported] = useState<Map<string, ImportedCell>>(new Map())
  const [skipped, setSkipped] = useState<Map<string, SkippedCell>>(new Map())
  const [failed, setFailed] = useState<Map<string, FailedCell>>(new Map())
  const [done, setDone] = useState<BatchDoneEvent | null>(null)

  useEffect(() => {
    let cancelled = false
    batchImported(batchId)
      .then((rows) => {
        if (cancelled) return
        putRows(rows)
        setImported((prev) => mergeRows(prev, rows))
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [batchId])

  useEffect(() => {
    let off1: UnlistenFn | undefined
    let off2: UnlistenFn | undefined
    ;(async () => {
      off1 = await listen<FileEvent>("ingest://file", (event) => {
        const ev = event.payload
        if (ev.batch_id !== batchId) return
        if (ev.state === "done" && ev.image_id && ev.content_hash) {
          setImported((prev) => {
            const next = new Map(prev)
            const existing = next.get(ev.image_id!)
            next.set(ev.image_id!, {
              key: ev.image_id!,
              id: ev.image_id,
              hash: ev.content_hash!,
              filename: ev.original_filename,
              status: existing?.status === "ready" ? "ready" : "thumbnailing",
            })
            return next
          })
        } else if (ev.state === "skipped_duplicate" && ev.content_hash) {
          setSkipped((prev) => {
            const next = new Map(prev)
            next.set(ev.source_path, {
              key: ev.source_path,
              hash: ev.content_hash!,
              filename: ev.original_filename,
              existingId: ev.image_id ?? "",
            })
            return next
          })
        } else if (ev.state === "failed") {
          setFailed((prev) => {
            const next = new Map(prev)
            next.set(ev.source_path, {
              key: ev.source_path,
              filename: ev.original_filename,
              sourcePath: ev.source_path,
              error: ev.error ?? "unknown",
            })
            return next
          })
        }
      })
      off2 = await listen<BatchDoneEvent>("ingest://batch-done", async (event) => {
        if (event.payload.batch_id !== batchId) return
        setDone(event.payload)
        try {
          const rows = await batchImported(batchId)
          putRows(rows)
          setImported((prev) => mergeRows(prev, rows))
        } catch {
          // ignore
        }
      })
    })()
    return () => {
      off1?.()
      off2?.()
    }
  }, [batchId])

  const skippedIds = useMemo(
    () => Array.from(skipped.values(), (c) => c.existingId).filter(Boolean),
    [skipped],
  )
  useEffect(() => {
    void loadMarks(skippedIds)
  }, [skippedIds])

  const importedList = useMemo(
    () => Array.from(imported.values()),
    [imported],
  )
  const skippedList = useMemo(() => Array.from(skipped.values()), [skipped])
  const failedList = useMemo(() => Array.from(failed.values()), [failed])

  return (
    <section className="mt-4">
      <h2 className="my-2 flex items-baseline gap-2 font-heading text-base font-medium">
        Last import
        {done && (
          <span className="text-sm font-normal text-muted-foreground">
            Imported {done.imported} · Skipped {done.skipped} · Failed{" "}
            {done.failed}
          </span>
        )}
      </h2>

      <Section label={`Imported (${importedList.length})`}>
        <ThumbGrid
          items={importedList}
          cellSize={cellSize}
          renderCell={(it) => (
            <MarkedImageCard
              id={it.key}
              hash={it.hash}
              filename={it.filename}
              status={it.status}
            />
          )}
        />
      </Section>

      <Section label={`Skipped — already in catalog (${skippedList.length})`}>
        <ThumbGrid
          items={skippedList}
          cellSize={cellSize}
          renderCell={(it) =>
            // A duplicate is the catalog's existing image: its marks are that image's.
            it.existingId ? (
              <MarkedImageCard id={it.existingId} hash={it.hash} filename={it.filename} status="ready" />
            ) : (
              <ImageCard hash={it.hash} filename={it.filename} status="ready" />
            )
          }
        />
      </Section>

      <Section label={`Failed (${failedList.length})`}>
        <ul className="m-0 list-disc pl-5">
          {failedList.map((f) => (
            <li key={f.key} className="text-xs">
              <code className="select-text rounded-sm bg-muted px-1 py-0.5 text-[11px]">
                {f.filename}
              </code>{" "}
              — <span className="select-text">{f.error}</span>{" "}
              <span className="text-muted-foreground">({f.sourcePath})</span>
            </li>
          ))}
        </ul>
      </Section>
    </section>
  )
}

function mergeRows(prev: Map<string, ImportedCell>, rows: ImportedRow[]) {
  const next = new Map(prev)
  for (const row of rows) {
    next.set(row.id, {
      key: row.id,
      id: row.id,
      hash: row.content_hash,
      filename: row.title,
      status: row.thumbnails_status,
    })
  }
  return next
}

function Section({
  label,
  children,
}: {
  label: string
  children: React.ReactNode
}) {
  return (
    <div className="mb-6">
      <h3 className="my-2 font-heading text-[11px] font-semibold tracking-[0.08em] uppercase text-muted-foreground">
        {label}
      </h3>
      {children}
    </div>
  )
}
