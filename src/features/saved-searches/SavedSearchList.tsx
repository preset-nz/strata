import { MagnifyingGlass, X } from "@phosphor-icons/react"
import { useEffect, useRef, useState } from "react"

import { cn } from "@/lib/utils"
import type { SavedSearch } from "./api"

type Props = {
  searches: SavedSearch[]
  activeId: string | null
  disabled?: boolean
  onApply: (search: SavedSearch) => void
  onDelete: (search: SavedSearch) => void
  /** When set, a name field is shown, prefilled with this. */
  naming: string | null
  onName: (name: string) => void
  onCancelNaming: () => void
}

export function SavedSearchList({
  searches,
  activeId,
  disabled,
  onApply,
  onDelete,
  naming,
  onName,
  onCancelNaming,
}: Props) {
  return (
    <div className="flex flex-col gap-1">
      {naming !== null && (
        <NameField initial={naming} onName={onName} onCancel={onCancelNaming} />
      )}
      {searches.length === 0 && naming === null ? (
        <p className="px-1 text-muted-foreground/70">
          Search or filter, then Edit › Save Search…
        </p>
      ) : (
        <ul className="flex flex-col">
          {searches.map((s) => (
            <li key={s.id} className="group relative">
              <button
                type="button"
                aria-pressed={activeId === s.id}
                disabled={disabled}
                onClick={() => onApply(s)}
                className={cn(
                  "flex w-full items-center gap-1.5 rounded-sm px-1 py-0.5 pr-5 text-left transition-colors",
                  "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                  activeId === s.id
                    ? "bg-muted text-foreground"
                    : "text-foreground hover:bg-muted/60",
                  disabled && "cursor-default opacity-60 hover:bg-transparent"
                )}
              >
                <MagnifyingGlass
                  weight="bold"
                  className="size-3 shrink-0 text-muted-foreground"
                />
                <span className="truncate">{s.name}</span>
              </button>
              <button
                type="button"
                aria-label={`Delete saved search ${s.name}`}
                title="Delete"
                disabled={disabled}
                onClick={() => onDelete(s)}
                className="absolute top-1/2 right-0.5 hidden -translate-y-1/2 rounded-sm p-0.5 text-muted-foreground hover:text-foreground focus-visible:block group-hover:block"
              >
                <X weight="bold" className="size-3" />
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

function NameField({
  initial,
  onName,
  onCancel,
}: {
  initial: string
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
      aria-label="Name for the saved search"
      placeholder="Name this search"
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
      className="h-6 rounded-sm border border-ring bg-transparent px-1 text-xs outline-none"
    />
  )
}
