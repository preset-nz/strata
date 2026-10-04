import { useMemo, useState } from "react"

import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import type { PrescanSummary } from "./api"
import { allFolders, boxState, selectionTotals, toggleFolder } from "./folderSelection"

type Props = {
  scan: PrescanSummary
  onConfirm: (folders: string[]) => void
  onCancel: () => void
  busy?: boolean
}

function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`
}

export function ImportConfirmation({ scan, onConfirm, onCancel, busy }: Props) {
  const [kept, setKept] = useState(() => allFolders(scan.folders))
  const { images, folders } = useMemo(() => selectionTotals(scan.folders, kept), [scan.folders, kept])

  const breakdown = Object.entries(scan.by_extension)
    .map(([ext, n]) => `${n} ${ext}`)
    .join(", ")
  const selection = `${plural(images, "image", "images")} from ${plural(folders, "folder", "folders")}`

  return (
    <div className="rounded-md border border-border p-4">
      <p className="text-sm">
        <strong className="font-medium">{scan.total}</strong> images found in{" "}
        <code className="select-text rounded-sm bg-muted px-1 py-0.5 text-xs">
          {scan.root}
        </code>
      </p>
      {breakdown && (
        <p className="mt-1 text-xs text-muted-foreground">{breakdown}</p>
      )}

      {scan.folders.length > 1 && (
        <ul className="mt-3 max-h-72 overflow-y-auto rounded-sm border border-border py-1 text-sm">
          {scan.folders.map((row) => {
            const state = boxState(scan.folders, kept, row.path)
            return (
              <li key={row.path}>
                <label
                  className="flex cursor-default items-center gap-2 px-2 py-0.5 hover:bg-muted"
                  style={{ paddingLeft: `${0.5 + row.depth * 1.25}rem` }}
                >
                  <Checkbox
                    checked={state === "checked"}
                    indeterminate={state === "mixed"}
                    onCheckedChange={() => setKept((k) => toggleFolder(scan.folders, k, row.path))}
                    disabled={busy}
                  />
                  <span className="min-w-0 flex-1 truncate">{row.name}</span>
                  <span className="text-xs tabular-nums text-muted-foreground">
                    {row.own > 0 ? row.own : ""}
                  </span>
                </label>
              </li>
            )
          })}
        </ul>
      )}

      <p className="mt-3 text-sm">
        About to import {selection} into Strata.{" "}
        {scan.keep_source_files
          ? "Source files stay where they are."
          : `Those ${plural(images, "source file", "source files")} will be moved to the Trash after each successful import.`}{" "}
        Unchecked folders and non-image files are left untouched. Continue?
      </p>
      <div className="mt-4 flex gap-2">
        <Button onClick={() => onConfirm([...kept])} disabled={busy || images === 0}>
          Confirm import
        </Button>
        <Button variant="outline" onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
      </div>
    </div>
  )
}
