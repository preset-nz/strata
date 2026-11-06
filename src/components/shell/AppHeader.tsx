import { Plus } from "@phosphor-icons/react"
import { LibraryHeader } from "@preset.nz/ux-kit"
import type { RefObject } from "react"
import { Button } from "@/components/ui/button"

type Props = {
  total: number
  filtered: number | null
  activeFilterCount: number
  onAdd: () => void
  addDisabled?: boolean
  onBack?: () => void
  leftCollapsed: boolean
  onToggleLeft: () => void
  rightCollapsed: boolean
  onToggleRight: () => void
  search: string
  onSearchChange: (next: string) => void
  searchRef: RefObject<HTMLInputElement | null>
}

// Strata's header: the library shell's strip, with Strata's summary line, its
// search wording and its Add button.
export function AppHeader({
  total,
  filtered,
  activeFilterCount,
  onAdd,
  addDisabled,
  onBack,
  leftCollapsed,
  onToggleLeft,
  rightCollapsed,
  onToggleRight,
  search,
  onSearchChange,
  searchRef,
}: Props) {
  const summary = (() => {
    if (total === 0) return "Library"
    if (filtered != null && filtered !== total) {
      return `Library · ${filtered.toLocaleString()} of ${total.toLocaleString()}`
    }
    return `Library · ${total.toLocaleString()}`
  })()

  return (
    <LibraryHeader
      title="Strata"
      summary={
        <>
          <span className="text-muted-foreground text-xs">{summary}</span>
          {activeFilterCount > 0 && (
            <span className="text-muted-foreground text-xs">
              · {activeFilterCount} active{" "}
              {activeFilterCount === 1 ? "filter" : "filters"}
            </span>
          )}
        </>
      }
      left={{
        collapsed: leftCollapsed,
        onToggle: onToggleLeft,
        label: "filters panel",
        shortcut: "[",
      }}
      right={{
        collapsed: rightCollapsed,
        onToggle: onToggleRight,
        label: "properties panel",
        shortcut: "]",
      }}
      onBack={onBack}
      search={{
        value: search,
        onChange: onSearchChange,
        inputRef: searchRef,
        placeholder: "Search prompts, keywords, names",
        label: "Search the library",
        shortcut: "⌘F",
      }}
    >
      <Button
        size="sm"
        variant="outline"
        onClick={onAdd}
        disabled={addDisabled}
      >
        <Plus weight="bold" />
        Add
      </Button>
    </LibraryHeader>
  )
}
