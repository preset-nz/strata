import type { RefObject } from "react"
import { Plus, ArrowLeft, SidebarSimple, MagnifyingGlass } from "@phosphor-icons/react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"

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
    <header className="flex h-10 shrink-0 items-center gap-3 border-b border-border px-3">
      <Button
        size="icon-sm"
        variant="ghost"
        onClick={onToggleLeft}
        aria-pressed={!leftCollapsed}
        aria-label={leftCollapsed ? "Show filters panel" : "Hide filters panel"}
        title={`${leftCollapsed ? "Show" : "Hide"} filters panel  [`}
      >
        <SidebarSimple weight="bold" />
      </Button>
      {onBack && (
        <Button size="sm" variant="ghost" onClick={onBack} aria-label="Back to library">
          <ArrowLeft weight="bold" />
          Library
        </Button>
      )}
      <h1 className="font-heading text-base font-semibold tracking-[0.04em] uppercase">
        Strata
      </h1>
      <span className="text-xs text-muted-foreground">{summary}</span>
      {activeFilterCount > 0 && (
        <span className="text-xs text-muted-foreground">
          · {activeFilterCount} active{" "}
          {activeFilterCount === 1 ? "filter" : "filters"}
        </span>
      )}
      <div className="ml-auto flex items-center gap-2">
        <div className="relative w-56">
          <MagnifyingGlass
            weight="bold"
            className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            ref={searchRef}
            type="search"
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.stopPropagation()
                onSearchChange("")
                e.currentTarget.blur()
              }
            }}
            placeholder="Search prompts, keywords, names"
            aria-label="Search the library"
            title="Search the library  ⌘F"
            className="pl-7"
          />
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={onAdd}
          disabled={addDisabled}
        >
          <Plus weight="bold" />
          Add
        </Button>
        <Button
          size="icon-sm"
          variant="ghost"
          onClick={onToggleRight}
          aria-pressed={!rightCollapsed}
          aria-label={rightCollapsed ? "Show properties panel" : "Hide properties panel"}
          title={`${rightCollapsed ? "Show" : "Hide"} properties panel  ]`}
        >
          <SidebarSimple weight="bold" className="scale-x-[-1]" />
        </Button>
      </div>
    </header>
  )
}
