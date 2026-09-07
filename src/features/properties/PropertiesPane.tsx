import { useMemo } from "react"
import { PropertyPanel, registerBuiltinRenderers } from "@preset.nz/property-editors"
import { useSelection } from "@/stores/selection"
import { registerStrataRenderers } from "./renderers"
import { registerStrataScopes } from "./scopes"
import { useImageDetails } from "./useImageDetails"
import { useBatchDetails } from "./useBatchDetails"

registerBuiltinRenderers()
registerStrataRenderers()
registerStrataScopes()

const EMPTY_CTX = Object.freeze({})

export function PropertiesPane() {
  const { selection } = useSelection()

  const imageId = selection.kind === "image" ? selection.id : null
  const batchId = selection.kind === "batch" ? selection.id : null

  const details = useImageDetails(imageId)
  const batch = useBatchDetails(batchId)

  const title = useMemo(() => {
    if (selection.kind === "image") return "Image"
    if (selection.kind === "batch") return "Batch"
    return "Properties"
  }, [selection.kind])

  const ctx = useMemo(() => {
    if (selection.kind === "image") return details ? { details } : EMPTY_CTX
    if (selection.kind === "batch") return batch ? { batch } : EMPTY_CTX
    return EMPTY_CTX
  }, [selection.kind, details, batch])

  return (
    <aside
      data-testid="properties-pane"
      className="flex min-h-0 flex-1 flex-col bg-background"
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
            ctx={ctx}
            readOnly
          />
        )}
      </div>
    </aside>
  )
}
