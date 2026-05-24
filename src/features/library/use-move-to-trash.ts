import { useCallback } from "react"
import { useSnackbar } from "@/components/ui/snackbar"
import { deleteImages, restoreImages } from "./api"

/**
 * Centralised soft-delete orchestration. Both the right-click "Move to Trash"
 * and the drag-onto-Trash-rail gestures end here. UI-side state (optimistic
 * removal, count refresh) is driven by the `library://images-trashed` and
 * `library://images-restored` Tauri events that the backend emits — this hook
 * stays focused on the API call and the Undo snackbar.
 */
export function useMoveToTrash() {
  const { show } = useSnackbar()
  return useCallback(
    async (ids: string[]) => {
      if (ids.length === 0) return
      try {
        await deleteImages(ids)
        const count = ids.length
        const message =
          count === 1 ? "Moved to Trash" : `Moved ${count} to Trash`
        show({
          message,
          action: {
            label: "Undo",
            onClick: () => {
              void restoreImages(ids)
            },
          },
        })
      } catch (e) {
        show({ message: `Could not delete: ${String(e)}` })
      }
    },
    [show],
  )
}
