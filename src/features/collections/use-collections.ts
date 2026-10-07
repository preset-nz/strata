import { undo } from "@preset.nz/app-kit/core"
import { listen } from "@tauri-apps/api/event"
import { useCallback, useEffect, useState } from "react"

import { useSnackbar } from "@/components/ui/use-snackbar"
import { HISTORY_CHANGED } from "@/features/curation/marks"
import {
  addToCollection,
  type Collection,
  createCollection,
  deleteCollection,
  listCollections,
  removeFromCollection,
  renameCollection,
} from "./api"

/**
 * The collections, kept in step with the catalog, and their commands. Every
 * command is an undo step; `changed` is called after one lands so the host can
 * reload what it shows (counts, a grid filtered to the collection).
 */
export function useCollections(changed: () => void) {
  const [collections, setCollections] = useState<Collection[]>([])
  /** Images the name field will put in the new collection; null while not naming. */
  const [naming, setNaming] = useState<string[] | null>(null)
  const { show } = useSnackbar()

  const reload = useCallback(async () => {
    try {
      setCollections(await listCollections())
    } catch (e) {
      console.error("collections failed to load:", e)
    }
  }, [])

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- initial load
    void reload()
    const off = listen(HISTORY_CHANGED, () => void reload())
    return () => void off.then((u) => u())
  }, [reload])

  const run = useCallback(
    async (what: string, op: () => Promise<unknown>) => {
      try {
        await op()
        await reload()
        changed()
      } catch (e) {
        show({ message: `Couldn't ${what}: ${String(e)}` })
      }
    },
    [reload, changed, show]
  )

  return {
    collections,
    naming,
    /** Opens the name field; the new collection will hold `images`. */
    startNaming: useCallback((images: string[]) => setNaming(images), []),
    cancelNaming: useCallback(() => setNaming(null), []),
    create: useCallback(
      (name: string) => {
        const images = naming ?? []
        setNaming(null)
        return run("create the collection", () =>
          createCollection(name, images)
        )
      },
      [naming, run]
    ),
    rename: useCallback(
      (id: string, name: string) =>
        run("rename the collection", () => renameCollection(id, name)),
      [run]
    ),
    remove: useCallback(
      (c: Collection) =>
        run("delete the collection", async () => {
          await deleteCollection(c.id)
          show({
            message: `Deleted “${c.name}”`,
            action: { label: "Undo", onClick: () => void undo() },
          })
        }),
      [run, show]
    ),
    add: useCallback(
      (id: string, images: string[]) =>
        run("add to the collection", () => addToCollection(id, images)),
      [run]
    ),
    removeImages: useCallback(
      (id: string, images: string[]) =>
        run("remove from the collection", () =>
          removeFromCollection(id, images)
        ),
      [run]
    ),
  }
}
