import { useEffect } from "react"

type Props = {
  onDropped: (path: string) => void
}

/**
 * Window-level folder-drop handler. We disable Tauri's native drag-drop
 * interceptor (`dragDropEnabled: false` in tauri.conf.json) so that internal
 * HTML5 DnD inside the webview — drag-to-trash, future drag-out-as-file —
 * receives the standard dragenter/dragover/drop events without OS-level
 * interception. The trade-off is losing the convenient `onDragDropEvent`
 * native API; we reach for the path on the dropped File via the wry-exposed
 * non-standard `path` property (macOS only; behaves like Electron's File.path).
 *
 * The Add button stays the canonical way to ingest; this is the gesture
 * shortcut from Finder.
 */
type FileWithPath = File & { path?: string }

export function DropZone({ onDropped }: Props) {
  useEffect(() => {
    const onDragOver = (e: DragEvent) => {
      if (!e.dataTransfer) return
      if (!Array.from(e.dataTransfer.types).includes("Files")) return
      e.preventDefault()
      e.dataTransfer.dropEffect = "copy"
    }
    const onDrop = (e: DragEvent) => {
      const files = e.dataTransfer?.files
      if (!files || files.length === 0) return
      e.preventDefault()
      const first = files[0] as FileWithPath
      if (first.path) {
        onDropped(first.path)
      }
    }
    window.addEventListener("dragover", onDragOver)
    window.addEventListener("drop", onDrop)
    return () => {
      window.removeEventListener("dragover", onDragOver)
      window.removeEventListener("drop", onDrop)
    }
  }, [onDropped])
  return null
}
