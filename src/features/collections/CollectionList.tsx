import { useEffect, useRef, useState } from "react"
import { Stack, X } from "@phosphor-icons/react"

import { readStrataImagePayload } from "@/components/image-card/use-draggable-card"
import { cn } from "@/lib/utils"
import type { Collection } from "./api"

type Props = {
  collections: Collection[]
  selectedId: string | null
  disabled?: boolean
  onSelect: (id: string | null) => void
  onRename: (id: string, name: string) => void
  onDelete: (c: Collection) => void
  /** Cards dropped on a row. */
  onDrop: (id: string, images: string[]) => void
  /** Set while a new collection is being named. */
  naming: boolean
  onName: (name: string) => void
  onCancelNaming: () => void
}

export function CollectionList({
  collections,
  selectedId,
  disabled,
  onSelect,
  onRename,
  onDelete,
  onDrop,
  naming,
  onName,
  onCancelNaming,
}: Props) {
  const [renaming, setRenaming] = useState<string | null>(null)
  const [dropTarget, setDropTarget] = useState<string | null>(null)
  return (
    <div className="flex flex-col gap-1">
      {naming && <NameField initial="" placeholder="Name the collection" onName={onName} onCancel={onCancelNaming} />}
      {collections.length === 0 && !naming ? (
        <p className="px-1 text-muted-foreground/70">Image › New Collection…, then drag images onto it</p>
      ) : (
        <ul className="flex flex-col">
          {collections.map((c) =>
            renaming === c.id ? (
              <li key={c.id}>
                <NameField
                  initial={c.name}
                  placeholder="Collection name"
                  onName={(name) => {
                    setRenaming(null)
                    onRename(c.id, name)
                  }}
                  onCancel={() => setRenaming(null)}
                />
              </li>
            ) : (
              <li key={c.id} className="group relative">
                <button
                  type="button"
                  aria-pressed={selectedId === c.id}
                  disabled={disabled}
                  title="Double-click to rename; drop images here to add them"
                  onClick={() => onSelect(selectedId === c.id ? null : c.id)}
                  onDoubleClick={() => setRenaming(c.id)}
                  // WebKit hides dataTransfer.types during dragover, so accept
                  // any drag and check the payload on drop (as the Trash row does).
                  onDragOver={(e) => {
                    e.preventDefault()
                    e.dataTransfer.dropEffect = "copy"
                    setDropTarget(c.id)
                  }}
                  onDragLeave={() => setDropTarget((t) => (t === c.id ? null : t))}
                  onDrop={(e) => {
                    e.preventDefault()
                    setDropTarget(null)
                    const ids = readStrataImagePayload(e.dataTransfer)
                    if (ids?.length) onDrop(c.id, ids)
                  }}
                  className={cn(
                    "flex w-full items-center gap-1.5 rounded-sm px-1 py-0.5 pr-5 text-left transition-colors",
                    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                    selectedId === c.id ? "bg-muted text-foreground" : "text-foreground hover:bg-muted/60",
                    dropTarget === c.id && "ring-1 ring-ring",
                    disabled && "cursor-default opacity-60 hover:bg-transparent",
                  )}
                >
                  <Stack weight="bold" className="size-3 shrink-0 text-muted-foreground" />
                  <span className="flex-1 truncate">{c.name}</span>
                  <span className="text-[10px] tabular-nums text-muted-foreground group-hover:invisible">
                    {c.count.toLocaleString()}
                  </span>
                </button>
                <button
                  type="button"
                  aria-label={`Delete collection ${c.name}`}
                  title="Delete (Edit › Undo brings it back)"
                  disabled={disabled}
                  onClick={() => onDelete(c)}
                  className="absolute top-1/2 right-0.5 hidden -translate-y-1/2 rounded-sm p-0.5 text-muted-foreground group-hover:block hover:text-foreground focus-visible:block"
                >
                  <X weight="bold" className="size-3" />
                </button>
              </li>
            ),
          )}
        </ul>
      )}
    </div>
  )
}

function NameField({
  initial,
  placeholder,
  onName,
  onCancel,
}: {
  initial: string
  placeholder: string
  onName: (name: string) => void
  onCancel: () => void
}) {
  const [value, setValue] = useState(initial)
  const ref = useRef<HTMLInputElement>(null)
  useEffect(() => {
    ref.current?.focus()
    ref.current?.select()
  }, [])
  return (
    <input
      ref={ref}
      value={value}
      aria-label={placeholder}
      placeholder={placeholder}
      onChange={(e) => setValue(e.target.value)}
      onBlur={onCancel}
      onKeyDown={(e) => {
        if (e.key === "Enter" && value.trim()) {
          e.preventDefault()
          onName(value.trim())
        } else if (e.key === "Escape") {
          e.stopPropagation()
          onCancel()
        }
      }}
      className="h-6 w-full rounded-sm border border-ring bg-transparent px-1 text-xs outline-none"
    />
  )
}
