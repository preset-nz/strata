import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { useEffect, useState } from "react"

type ProgressEvent = {
  batch_id: string
  total: number
  processed: number
  imported: number
  skipped: number
  failed: number
}

type BatchDoneEvent = {
  batch_id: string
  imported: number
  skipped: number
  failed: number
}

type Props = {
  batchId: string
  total: number
}

export function JobProgress({ batchId, total }: Props) {
  // Each event carries the running totals, so one missed while this mounted
  // costs nothing: the next one corrects the count.
  const [progress, setProgress] = useState<ProgressEvent | null>(null)
  const [doneEvent, setDoneEvent] = useState<BatchDoneEvent | null>(null)

  useEffect(() => {
    let off1: UnlistenFn | undefined
    let off2: UnlistenFn | undefined
    let cancelled = false
    ;(async () => {
      const a = await listen<ProgressEvent>("ingest://progress", (event) => {
        if (event.payload.batch_id === batchId) setProgress(event.payload)
      })
      const b = await listen<BatchDoneEvent>("ingest://batch-done", (event) => {
        if (event.payload.batch_id === batchId) setDoneEvent(event.payload)
      })
      if (cancelled) {
        a()
        b()
        return
      }
      off1 = a
      off2 = b
    })()
    return () => {
      cancelled = true
      off1?.()
      off2?.()
    }
  }, [batchId])

  const processed = progress?.processed ?? 0

  return (
    <section className="mt-4">
      <h2 className="mb-1 font-heading font-medium text-base">
        Batch {batchId.slice(0, 8)}
      </h2>
      {doneEvent ? (
        <p className="text-muted-foreground text-sm">
          Imported {doneEvent.imported} · Skipped {doneEvent.skipped} · Failed{" "}
          {doneEvent.failed}
        </p>
      ) : (
        <p className="text-muted-foreground text-sm">
          {processed} of {total} · Imported {progress?.imported ?? 0} · Skipped{" "}
          {progress?.skipped ?? 0} · Failed {progress?.failed ?? 0}
        </p>
      )}
    </section>
  )
}
