import { useMemo } from "react"
import { PropertyPanel, registerBuiltinRenderers } from "@/properties"
import { useSelection } from "@/stores/selection"
import { registerStrataRenderers } from "./renderers"
import { registerStrataScopes } from "./scopes"

// Run at module import time: renderers and scopes are registered before
// any component that consumes them renders.
registerBuiltinRenderers()
registerStrataRenderers()
registerStrataScopes()

// Strata's scopes need no ctx today (read functions close over the selection).
// Hoisted so PropertyPanel's read-memo deps stay stable across renders.
const EMPTY_CTX = Object.freeze({})

export function PropertiesPane() {
  const { selection } = useSelection()

  const title = useMemo(() => {
    if (selection.kind === "image") return "Image"
    if (selection.kind === "batch") return "Batch"
    return "Properties"
  }, [selection.kind])

  return (
    <aside
      data-testid="properties-pane"
      className="flex h-full w-80 shrink-0 flex-col border-l border-border bg-background"
    >
      <header className="flex h-9 shrink-0 items-center border-b border-border px-3">
        <h2 className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          {title}
        </h2>
      </header>
      <div className="flex-1 overflow-auto">
        {selection.kind === "none" ? (
          <div className="flex h-full items-center justify-center p-6 text-center">
            <p className="text-xs text-muted-foreground">
              Select an image or a batch to see its properties.
            </p>
          </div>
        ) : (
          <PropertyPanel
            scopeKey={selection.kind}
            selection={selection}
            ctx={EMPTY_CTX}
            readOnly
          />
        )}
      </div>
    </aside>
  )
}
