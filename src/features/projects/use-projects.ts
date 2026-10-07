import { listen } from "@tauri-apps/api/event"
import { useCallback, useEffect, useState } from "react"

import { useSnackbar } from "@/components/ui/use-snackbar"
import { HISTORY_CHANGED } from "@/features/curation/marks"
import {
  addToProject,
  createProject,
  listProjects,
  type Project,
  removeFromProject,
  setProjectArchived,
  setProjectFavourite,
} from "./api"

/**
 * The work projects found on disk, with Strata's curation, and their
 * commands. Every curation command is an undo step; `changed` runs after one
 * lands. The list is read again when the window comes back to the front, so a
 * project made in Shard or Oblique shows up.
 */
export function useProjects(changed: () => void) {
  const [projects, setProjects] = useState<Project[]>([])
  const [creating, setCreating] = useState(false)
  const { show } = useSnackbar()

  const reload = useCallback(async () => {
    try {
      setProjects(await listProjects())
    } catch (e) {
      console.error("projects failed to load:", e)
    }
  }, [])

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- initial load
    void reload()
    const off = listen(HISTORY_CHANGED, () => void reload())
    const onFocus = () => void reload()
    window.addEventListener("focus", onFocus)
    return () => {
      void off.then((u) => u())
      window.removeEventListener("focus", onFocus)
    }
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
    projects,
    creating,
    startCreating: useCallback(() => setCreating(true), []),
    cancelCreating: useCallback(() => setCreating(false), []),
    /** Resolves with the new key, or null when it failed (and said why). */
    create: useCallback(
      async (name: string, description: string): Promise<string | null> => {
        try {
          const key = await createProject(name, description)
          setCreating(false)
          await reload()
          return key
        } catch (e) {
          show({ message: `Couldn't create the project: ${String(e)}` })
          return null
        }
      },
      [reload, show]
    ),
    setFavourite: useCallback(
      (p: Project, on: boolean) =>
        run("change the project", () => setProjectFavourite(p.key, on)),
      [run]
    ),
    setArchived: useCallback(
      (p: Project, on: boolean) =>
        run("change the project", () => setProjectArchived(p.key, on)),
      [run]
    ),
    add: useCallback(
      (key: string, images: string[]) =>
        run("add to the project", () => addToProject(key, images)),
      [run]
    ),
    removeImages: useCallback(
      (key: string, images: string[]) =>
        run("remove from the project", async () => {
          const removed = await removeFromProject(key, images)
          if (!removed)
            show({
              message:
                "Images under the project's folder stay in it. Move the files to take them out.",
            })
        }),
      [run, show]
    ),
  }
}
