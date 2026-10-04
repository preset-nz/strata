import { useCallback, useEffect, useMemo, useState } from "react"
import { listen } from "@tauri-apps/api/event"

import { useSnackbar } from "@/components/ui/use-snackbar"
import { deleteSearch, listSavedSearches, restoreSearch, saveSearch, type SavedSearch } from "./api"
import { isSavable, parseQuery, sameQuery, suggestName, type SavedQuery } from "./query"

/**
 * The saved searches, which one the library currently shows, and the save /
 * apply / delete commands. Save is Edit › Save Search…; the rail asks for a
 * name. Delete offers Undo the way Move to Trash does.
 */
export function useSavedSearches(current: SavedQuery, apply: (q: SavedQuery) => void) {
  const [searches, setSearches] = useState<SavedSearch[]>([])
  const [naming, setNaming] = useState<string | null>(null)
  const { show } = useSnackbar()

  const reload = useCallback(async () => {
    try {
      setSearches(await listSavedSearches())
    } catch (e) {
      console.error("saved searches failed to load:", e)
    }
  }, [])

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- initial load
    void reload()
  }, [reload])

  const startNaming = useCallback(() => {
    if (!isSavable(current)) {
      show({ message: "Search or pick a filter first, then save it." })
      return
    }
    setNaming(suggestName(current))
  }, [current, show])

  useEffect(() => {
    let off: (() => void) | undefined
    let cancelled = false
    void listen("menu://edit/save-search", startNaming).then((u) => {
      if (cancelled) u()
      else off = u
    })
    return () => {
      cancelled = true
      off?.()
    }
  }, [startNaming])

  const save = useCallback(
    async (name: string) => {
      setNaming(null)
      try {
        await saveSearch(name, current)
        await reload()
      } catch (e) {
        show({ message: `Couldn't save the search: ${String(e)}` })
      }
    },
    [current, reload, show],
  )

  const remove = useCallback(
    async (s: SavedSearch) => {
      try {
        const gone = await deleteSearch(s.id)
        await reload()
        if (gone) {
          show({
            message: `Deleted "${gone.name}"`,
            action: {
              label: "Undo",
              onClick: () => void restoreSearch(gone).then(reload),
            },
          })
        }
      } catch (e) {
        show({ message: `Couldn't delete the search: ${String(e)}` })
      }
    },
    [reload, show],
  )

  const applySearch = useCallback(
    (s: SavedSearch) => {
      const q = parseQuery(s.query)
      if (q) apply(q)
      else show({ message: `"${s.name}" was saved by a newer Strata and can't run here.` })
    },
    [apply, show],
  )

  const activeId = useMemo(() => {
    if (!isSavable(current)) return null
    return searches.find((s) => {
      const q = parseQuery(s.query)
      return q !== null && sameQuery(q, current)
    })?.id ?? null
  }, [searches, current])

  return {
    searches,
    activeId,
    naming,
    startNaming,
    save,
    cancelNaming: () => setNaming(null),
    remove,
    applySearch,
  }
}
