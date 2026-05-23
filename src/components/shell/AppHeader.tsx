import { Plus } from "@phosphor-icons/react"
import { Button } from "@/components/ui/button"

type Props = {
  total: number
  filtered: number | null
  activeFilterCount: number
  onAdd: () => void
  addDisabled?: boolean
}

export function AppHeader({
  total,
  filtered,
  activeFilterCount,
  onAdd,
  addDisabled,
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
      <h1 className="font-heading text-sm font-medium tracking-wide">Strata</h1>
      <span className="text-xs text-muted-foreground">{summary}</span>
      {activeFilterCount > 0 && (
        <span className="text-xs text-muted-foreground">
          · {activeFilterCount} active{" "}
          {activeFilterCount === 1 ? "filter" : "filters"}
        </span>
      )}
      <div className="ml-auto">
        <Button
          size="sm"
          variant="outline"
          onClick={onAdd}
          disabled={addDisabled}
        >
          <Plus weight="bold" />
          Add
        </Button>
      </div>
    </header>
  )
}
