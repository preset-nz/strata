import { Button } from "@/components/ui/button"
import type { PrescanSummary } from "./api"

type Props = {
  scan: PrescanSummary
  onConfirm: () => void
  onCancel: () => void
  busy?: boolean
}

export function ImportConfirmation({ scan, onConfirm, onCancel, busy }: Props) {
  const breakdown = Object.entries(scan.by_extension)
    .map(([ext, n]) => `${n} ${ext}`)
    .join(", ")
  return (
    <div className="rounded-md border border-border p-4">
      <p className="text-sm">
        <strong className="font-medium">{scan.total}</strong> images ready for
        ingress from{" "}
        <code className="select-text rounded-sm bg-muted px-1 py-0.5 text-xs">
          {scan.root}
        </code>
      </p>
      {breakdown && (
        <p className="mt-1 text-xs text-muted-foreground">{breakdown}</p>
      )}
      <p className="mt-3 text-sm">
        About to import {scan.total} images from{" "}
        <code className="select-text rounded-sm bg-muted px-1 py-0.5 text-xs">
          {scan.root}
        </code>{" "}
        into Strata. Source files will be moved to the Trash after each
        successful import. Non-image files will be left in place. Continue?
      </p>
      <div className="mt-4 flex gap-2">
        <Button onClick={onConfirm} disabled={busy || scan.total === 0}>
          Confirm import
        </Button>
        <Button variant="outline" onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
      </div>
    </div>
  )
}
