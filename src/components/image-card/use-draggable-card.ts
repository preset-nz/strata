import { useCallback, useMemo, useRef } from "react"
import { useSelection } from "@/stores/selection"

/**
 * Drag MIME type carried in dataTransfer when a card is dragged.
 * Payload is a JSON-encoded array of image ids: `string[]`.
 *
 * Defined here so drop targets (Trash row, future epic 09 file-handoff) can
 * import the same constant and key off it without restating the contract.
 */
export const STRATA_IMAGE_MIME = "application/x-strata-image"

type DragProps = {
  draggable: true
  onDragStart: (e: React.DragEvent) => void
  onDragEnd: (e: React.DragEvent) => void
}

type Options = {
  /** Id of the card this draggable handle is attached to. */
  id: string
}

/**
 * Card-drag primitive. Returns props you spread onto the draggable element.
 *
 * Selection semantics: if the dragged id is part of the current selection, the
 * drag carries every selected id. Otherwise it carries only `id` and clears
 * selection (so the user isn't left with a stale highlight on something they
 * weren't operating on). Multi-select is currently single-image only — the
 * primitive is shaped for the eventual multi-select store without depending on
 * it today.
 */
export function useDraggableCard({ id }: Options): DragProps {
  const draggingRef = useRef(false)

  const onDragStart = useCallback(
    (e: React.DragEvent) => {
      const sel = useSelection.getState().selection
      const selectedIds = sel.kind === "image" ? [sel.id] : []
      const inSelection = selectedIds.includes(id)
      const ids = inSelection ? selectedIds : [id]
      if (!inSelection && selectedIds.length > 0) {
        useSelection.getState().clear()
      }

      const payload = JSON.stringify(ids)
      e.dataTransfer.effectAllowed = "move"
      e.dataTransfer.setData(STRATA_IMAGE_MIME, payload)
      // Also set text/plain for native consumers (e.g. external drop targets in
      // a future epic 09 file-handoff). Plain ids only, no chrome.
      e.dataTransfer.setData("text/plain", ids.join("\n"))
      draggingRef.current = true
    },
    [id],
  )

  const onDragEnd = useCallback(() => {
    draggingRef.current = false
  }, [])

  return useMemo(
    () => ({ draggable: true, onDragStart, onDragEnd }),
    [onDragStart, onDragEnd],
  )
}

/**
 * Helper for drop-target side: parse the JSON payload off a DataTransfer.
 * Returns the id list, or null if the drop didn't come from a Strata card.
 */
export function readStrataImagePayload(
  dt: DataTransfer | null,
): string[] | null {
  if (!dt) return null
  const raw = dt.getData(STRATA_IMAGE_MIME)
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw)
    if (Array.isArray(parsed) && parsed.every((x) => typeof x === "string")) {
      return parsed
    }
  } catch {
    /* fallthrough */
  }
  return null
}
