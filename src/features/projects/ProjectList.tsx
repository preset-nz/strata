import { useState } from "react"
import { Archive, ArrowCounterClockwise, Folder, Star } from "@phosphor-icons/react"

import { readStrataImagePayload } from "@/components/image-card/use-draggable-card"
import { cn } from "@/lib/utils"
import type { Project } from "./api"

type Props = {
  projects: Project[]
  selectedKey: string | null
  disabled?: boolean
  onSelect: (key: string | null) => void
  onFavourite: (p: Project, on: boolean) => void
  onArchive: (p: Project, on: boolean) => void
  onDrop: (key: string, images: string[]) => void
}

/** Favourites first, then by name; archived ones behind a toggle. */
export function ProjectList({ projects, selectedKey, disabled, onSelect, onFavourite, onArchive, onDrop }: Props) {
  const [showArchived, setShowArchived] = useState(false)
  const [dropTarget, setDropTarget] = useState<string | null>(null)
  const live = projects.filter((p) => !p.archived)
  const archived = projects.filter((p) => p.archived)
  const ordered = [...live.filter((p) => p.favourite), ...live.filter((p) => !p.favourite)]
  const shown = showArchived ? [...ordered, ...archived] : ordered

  if (projects.length === 0) {
    return <p className="px-1 text-muted-foreground/70">File › New Project…, or make one in Shard or Oblique</p>
  }
  return (
    <div className="flex flex-col gap-1">
      <ul className="flex flex-col">
        {shown.map((p) => (
          <li key={p.key} className="group relative">
            <button
              type="button"
              aria-pressed={selectedKey === p.key}
              disabled={disabled}
              title={[p.description, p.folder].filter(Boolean).join("\n")}
              onClick={() => onSelect(selectedKey === p.key ? null : p.key)}
              onDragOver={(e) => {
                e.preventDefault()
                e.dataTransfer.dropEffect = "copy"
                setDropTarget(p.key)
              }}
              onDragLeave={() => setDropTarget((t) => (t === p.key ? null : t))}
              onDrop={(e) => {
                e.preventDefault()
                setDropTarget(null)
                const ids = readStrataImagePayload(e.dataTransfer)
                if (ids?.length) onDrop(p.key, ids)
              }}
              className={cn(
                "flex w-full items-center gap-1.5 rounded-sm px-1 py-0.5 pr-10 text-left transition-colors",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                selectedKey === p.key ? "bg-muted text-foreground" : "text-foreground hover:bg-muted/60",
                dropTarget === p.key && "ring-1 ring-ring",
                p.archived && "text-muted-foreground",
                disabled && "cursor-default opacity-60 hover:bg-transparent",
              )}
            >
              {p.favourite ? (
                <Star weight="fill" className="size-3 shrink-0 text-yellow-400" />
              ) : (
                <Folder weight="bold" className="size-3 shrink-0 text-muted-foreground" />
              )}
              <span className="flex-1 truncate">{p.name}</span>
              <span className="text-[10px] tabular-nums text-muted-foreground group-hover:invisible">
                {p.count.toLocaleString()}
              </span>
            </button>
            <span className="absolute top-1/2 right-0.5 hidden -translate-y-1/2 items-center gap-0.5 group-hover:flex">
              <button
                type="button"
                aria-label={p.favourite ? `Unfavourite ${p.name}` : `Favourite ${p.name}`}
                title={p.favourite ? "Unfavourite" : "Favourite (pinned to the top)"}
                disabled={disabled}
                onClick={() => onFavourite(p, !p.favourite)}
                className="rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
              >
                <Star weight={p.favourite ? "fill" : "bold"} className="size-3" />
              </button>
              <button
                type="button"
                aria-label={p.archived ? `Unarchive ${p.name}` : `Archive ${p.name}`}
                title={p.archived ? "Unarchive" : "Archive (hidden from this list)"}
                disabled={disabled}
                onClick={() => onArchive(p, !p.archived)}
                className="rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
              >
                {p.archived ? <ArrowCounterClockwise weight="bold" className="size-3" /> : <Archive weight="bold" className="size-3" />}
              </button>
            </span>
          </li>
        ))}
      </ul>
      {archived.length > 0 && (
        <button
          type="button"
          onClick={() => setShowArchived((v) => !v)}
          className="self-start px-1 text-[10px] text-muted-foreground hover:text-foreground"
        >
          {showArchived ? "Hide archived" : `Show archived (${archived.length})`}
        </button>
      )}
    </div>
  )
}
